use super::messages::{BackendCommand, BackendEvent};
use crate::downloader::{DownloadConfig, DownloadEngine};
use crate::extractor::{
    native::youtube::NativeYoutubeExtractor, Extractor, Format, HybridExtractor, VideoInfo,
    YtDlpExtractor,
};
use crate::gui::DownloadProgressData;
use crate::queue::{DownloadTask, EventLog, QueueManager, TaskStatus};
use crate::utils::config::AppSettings;
use crate::utils::{get_app_support_dir, FileOrganizer, MetadataManager, OrganizationSettings};
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;

pub struct BackendActor {
    receiver: mpsc::Receiver<BackendCommand>,
    sender: mpsc::Sender<BackendEvent>,

    // Components
    extractor: Arc<HybridExtractor>,
    queue_manager: Arc<QueueManager>,
}

impl BackendActor {
    pub async fn new(
        settings: AppSettings,
        receiver: mpsc::Receiver<BackendCommand>,
        sender: mpsc::Sender<BackendEvent>,
    ) -> Result<Self> {
        // Initialize components
        // 1. Initialize Extractors
        let ytdlp = Arc::new(YtDlpExtractor::new()?);
        let native_youtube = Arc::new(NativeYoutubeExtractor::new());

        // 2. Build Hybrid Registry
        let extractors: Vec<Arc<dyn Extractor>> = vec![native_youtube];
        let fallback = ytdlp;
        let hybrid_extractor = Arc::new(HybridExtractor::new(extractors, fallback));
        let extractor = hybrid_extractor;

        let download_config = DownloadConfig {
            segments: settings.segments,
            connections_per_segment: 1,
            chunk_size: settings.chunk_size,
            retry_attempts: settings.retry_attempts,
            retry_delay: std::time::Duration::from_secs(2),
            enable_resume: settings.enable_resume,
            request_delay: std::time::Duration::from_millis(100),
        };

        let engine = DownloadEngine::new(download_config);

        let org_settings = OrganizationSettings::default();
        let file_organizer = FileOrganizer::new(org_settings)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize file organizer: {}", e))?;

        let metadata_manager = MetadataManager::new(&file_organizer.base_dir);
        let file_organizer = Arc::new(file_organizer);
        let metadata_manager = Arc::new(metadata_manager);

        // 3. Initialize Event Log
        let app_support_dir = get_app_support_dir();
        let event_log = Arc::new(
            EventLog::new(&app_support_dir)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize event log: {}", e))?,
        );

        let queue_manager = Arc::new(QueueManager::new(
            settings.max_concurrent,
            engine,
            (*file_organizer).clone(),
            (*metadata_manager).clone(),
            event_log,
        ));

        Ok(Self {
            receiver,
            sender,
            extractor,
            queue_manager,
        })
    }

    pub async fn run(mut self) {
        info!("BackendActor started");

        // Rehydrate persistence state
        if let Err(e) = self.queue_manager.rehydrate().await {
            tracing::error!("Failed to rehydrate queue state: {}", e);
        }

        // Spawn Queue Processor (independent loop)
        let qm_clone = self.queue_manager.clone();
        tokio::spawn(async move {
            qm_clone.start().await;
        });

        // Spawn Monitor (polls queue and sends events)
        // In a future refactor, QueueManager should push events directly.
        // For now, we port the polling logic to keep changes scoped.
        let qm_monitor = self.queue_manager.clone();
        let sender_monitor = self.sender.clone();
        tokio::spawn(async move {
            Self::monitor_loop(qm_monitor, sender_monitor).await;
        });

        while let Some(cmd) = self.receiver.recv().await {
            match cmd {
                BackendCommand::ExtractInfo { url } => {
                    self.handle_extract_info(url).await;
                }
                BackendCommand::StartDownload {
                    video_info,
                    output_path,
                    format_id,
                } => {
                    self.handle_start_download(video_info, output_path, format_id)
                        .await;
                }
                BackendCommand::PauseDownload(id) => {
                    let _ = self.queue_manager.pause_task(&id).await;
                    // confirmation sent via monitor loop
                }
                BackendCommand::ResumeDownload(id) => {
                    let _ = self.queue_manager.resume_task(&id).await;
                }
                BackendCommand::CancelDownload(id) => {
                    let _ = self.queue_manager.cancel_task(&id).await;
                }
                BackendCommand::RemoveTask(id) => {
                    let _ = self.queue_manager.remove_task(&id).await;
                }
                BackendCommand::ClearCompleted => {
                    let _ = self.queue_manager.clear_completed().await;
                }
                BackendCommand::ResumeAll => {
                    let _ = self.queue_manager.resume_all().await;
                }
                BackendCommand::Shutdown => {
                    info!("BackendActor shutting down");
                    break;
                }
            }
        }
    }

    async fn handle_extract_info(&self, url: String) {
        let _ = self.sender.send(BackendEvent::ExtractionStarted).await;

        match self.extractor.extract_info(&url).await {
            Ok(info) => {
                let _ = self
                    .sender
                    .send(BackendEvent::ExtractionCompleted(Ok(info)))
                    .await;
            }
            Err(e) => {
                let _ = self
                    .sender
                    .send(BackendEvent::ExtractionCompleted(Err(e.to_string())))
                    .await;
            }
        }
    }

    async fn handle_start_download(
        &self,
        video_info: VideoInfo,
        output_path: PathBuf,
        format_id: Option<String>,
    ) {
        // Validation and setup logic ported from BackendBridge

        // 1. Format Selection
        let format = match self.select_format(&video_info, format_id) {
            Ok(f) => f,
            Err(e) => {
                let _ = self.sender.send(BackendEvent::Error(e)).await;
                return;
            }
        };

        // 2. Get Direct URL
        let download_url = match self.get_download_url(&video_info, &format).await {
            Ok(url) => url,
            Err(e) => {
                let _ = self.sender.send(BackendEvent::Error(e)).await;
                return;
            }
        };

        // 3. Create Task
        let task_id = Uuid::new_v4().to_string();
        let mut updated_format = format.clone();
        updated_format.url = download_url;

        let task = DownloadTask {
            id: task_id.clone(),
            video_info: video_info.clone(),
            output_path: output_path.clone(), // Note: caller should handle path logic? Or we duplicate it here?
            // The original code did path fixing here. Let's do a basic fix if needed.
            format: updated_format,
            status: TaskStatus::Queued,
            progress: None,
            added_at: Utc::now(),
        };

        // 4. Add to Queue
        if let Err(e) = self.queue_manager.add_task(task).await {
            let _ = self
                .sender
                .send(BackendEvent::DownloadFailed {
                    task_id,
                    error: e.to_string(),
                })
                .await;
            return;
        }

        let _ = self
            .sender
            .send(BackendEvent::DownloadStarted {
                task_id,
                video_info,
            })
            .await;
    }

    fn select_format(
        &self,
        video_info: &VideoInfo,
        format_id: Option<String>,
    ) -> Result<Format, String> {
        // Logic from BackendBridge::start_download
        if let Some(id) = format_id {
            video_info
                .formats
                .iter()
                .find(|f| f.format_id == id)
                .cloned()
                .ok_or_else(|| "Format not found".to_string())
        } else {
            // combined format logic
            let combined_formats: Vec<_> = video_info
                .formats
                .iter()
                .filter(|f| {
                    let has_video = f.vcodec.as_deref().unwrap_or("none") != "none";
                    let has_audio = f.acodec.as_deref().unwrap_or("none") != "none";
                    let not_sb = !f.format_id.starts_with("sb");
                    has_video && has_audio && not_sb
                })
                .collect();

            if let Some(best) = combined_formats
                .iter()
                .max_by_key(|f| f.width.unwrap_or(0) * f.height.unwrap_or(0))
            {
                Ok((*best).clone())
            } else {
                Err("No combined format found".to_string())
            }
        }
    }

    async fn get_download_url(
        &self,
        video_info: &VideoInfo,
        format: &Format,
    ) -> Result<String, String> {
        let direct_url = self
            .extractor
            .get_direct_url(&video_info.url, &format.format_id)
            .await
            .map_err(|e| e.to_string())?;

        // HLS Check
        if direct_url.contains(".m3u8") || direct_url.contains("/manifest") {
            Ok(video_info.url.clone())
        } else {
            Ok(direct_url)
        }
    }

    async fn monitor_loop(qm: Arc<QueueManager>, sender: mpsc::Sender<BackendEvent>) {
        let mut last_statuses = std::collections::HashMap::new();

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Faster updates?

            let tasks = qm.get_all_tasks().await;

            for task in tasks {
                // Check status change
                let last_status = last_statuses
                    .get(&task.id)
                    .cloned()
                    .unwrap_or(TaskStatus::Queued);

                if task.status != last_status {
                    let status_str = match task.status {
                        TaskStatus::Queued => "Queued",
                        TaskStatus::Downloading => "Downloading",
                        TaskStatus::Paused => "Paused",
                        TaskStatus::Completed => "Completed",
                        TaskStatus::Failed(_) => "Failed",
                        TaskStatus::Cancelled => "Cancelled",
                    }
                    .to_string();

                    let _ = sender
                        .send(BackendEvent::TaskStatusUpdated {
                            task_id: task.id.clone(),
                            status: status_str.clone(),
                        })
                        .await;

                    if matches!(task.status, TaskStatus::Completed) {
                        let _ = sender
                            .send(BackendEvent::DownloadCompleted {
                                task_id: task.id.clone(),
                            })
                            .await;
                    }
                    if let TaskStatus::Failed(e) = &task.status {
                        let _ = sender
                            .send(BackendEvent::DownloadFailed {
                                task_id: task.id.clone(),
                                error: e.clone(),
                            })
                            .await;
                    }

                    last_statuses.insert(task.id.clone(), task.status.clone());
                }

                // Send Progress
                if let Some(progress) = &task.progress {
                    let data = DownloadProgressData {
                        progress: progress.percentage() as f32,
                        speed: progress.speed,
                        downloaded: progress.downloaded_bytes,
                        total: progress.total_bytes,
                        eta: progress.eta.map(|d| d.as_secs()),
                    };

                    // Only send if downloading or recently changed?
                    // To save bandwidth, maybe check if changed? For now send all.
                    let _ = sender
                        .send(BackendEvent::DownloadProgress {
                            task_id: task.id.clone(),
                            data,
                        })
                        .await;
                }
            }
        }
    }
}
