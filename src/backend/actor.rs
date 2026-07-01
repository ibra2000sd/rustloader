use super::messages::{BackendCommand, BackendEvent};
use crate::database::{DatabaseManager, DownloadRecord};
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
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

/// Map a queue `TaskStatus` to the `downloads` table's `(status, completed_at,
/// error_message)` columns. The single source of truth for that mapping, used
/// both for the DB history writes and the GUI's `TaskStatusUpdated` event, so
/// the two never drift apart.
fn task_status_db_fields(status: &TaskStatus) -> (String, Option<DateTime<Utc>>, Option<String>) {
    match status {
        TaskStatus::Queued => ("Queued".to_string(), None, None),
        TaskStatus::Downloading => ("Downloading".to_string(), None, None),
        TaskStatus::Paused => ("Paused".to_string(), None, None),
        TaskStatus::Completed => ("Completed".to_string(), Some(Utc::now()), None),
        TaskStatus::Failed(e) => ("Failed".to_string(), Some(Utc::now()), Some(e.clone())),
        TaskStatus::Cancelled => ("Cancelled".to_string(), Some(Utc::now()), None),
    }
}

pub struct BackendActor {
    receiver: mpsc::Receiver<BackendCommand>,
    sender: mpsc::Sender<BackendEvent>,

    // Components
    extractor: Arc<HybridExtractor>,
    queue_manager: Arc<QueueManager>,
    db_manager: Arc<DatabaseManager>,
}

impl BackendActor {
    pub async fn new(
        settings: AppSettings,
        receiver: mpsc::Receiver<BackendCommand>,
        sender: mpsc::Sender<BackendEvent>,
        db_manager: Arc<DatabaseManager>,
    ) -> Result<Self> {
        // Cookie source from settings, applied to both extraction and download
        // so authenticated sites (e.g. YouTube) work from the GUI.
        let cookies = crate::utils::CookieConfig::new(
            settings.cookies_from_browser.clone(),
            settings.cookies_file.clone(),
        );

        // Initialize components
        // 1. Initialize Extractors
        let ytdlp = Arc::new(YtDlpExtractor::new()?.with_cookies(cookies.clone()));
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

        let engine = DownloadEngine::new(download_config).with_ytdlp_options(
            crate::downloader::YtDlpOptions {
                cookies: cookies.clone(),
                ..Default::default()
            },
        );

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
            db_manager,
        })
    }

    /// All persisted download history (all-time, including completed/failed/
    /// cancelled downloads that may no longer be in the live queue),
    /// most-recent-first. Nothing renders this yet — the GUI history list is
    /// a follow-up (Shape-3 PR-2) — but the data is live and durable now.
    pub async fn download_history(&self) -> Result<Vec<DownloadRecord>> {
        self.db_manager.get_all_downloads().await
    }

    pub async fn run(mut self) {
        info!("BackendActor started");

        // Rehydrate persistence state
        if let Err(e) = self.queue_manager.rehydrate().await {
            tracing::error!("Failed to rehydrate queue state: {}", e);
        }

        // Load persisted download history (Shape-3 PR-1). This is separate
        // from the queue rehydrate above: rehydrate reconstructs the LIVE
        // queue's runtime state from the EventLog; this reads the `downloads`
        // table, the durable history of every download ever started,
        // including ones long since cleared from the live queue. Nothing
        // consumes it yet (that's the GUI history list, PR-2) — this just
        // proves it's live and durable across restarts.
        match self.db_manager.get_all_downloads().await {
            Ok(history) => info!(
                "Loaded {} historical download record(s) from the downloads table",
                history.len()
            ),
            Err(e) => warn!("Failed to load download history: {}", e),
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
        let db_monitor = Arc::clone(&self.db_manager);
        tokio::spawn(async move {
            Self::monitor_loop(qm_monitor, sender_monitor, db_monitor).await;
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
                    self.handle_start_download(*video_info, output_path, format_id)
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
        let format = match Self::select_format(&video_info, format_id) {
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

        // Persist the initial history row now that the task is actually
        // queued. Best-effort: a write failure here must not fail the
        // download — the queue (added above) is already the runtime source
        // of truth for this task; this is a durable log of it, not a second
        // one.
        let record = DownloadRecord {
            id: task_id.clone(),
            url: video_info.url.clone(),
            title: video_info.title.clone(),
            output_path: output_path.clone(),
            file_size: format.filesize,
            status: "Queued".to_string(),
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
        };
        if let Err(e) = self.db_manager.save_download(&record).await {
            warn!("Failed to persist download history for {}: {}", task_id, e);
        }

        let _ = self
            .sender
            .send(BackendEvent::DownloadStarted {
                task_id,
                video_info,
            })
            .await;
    }

    /// Choose the format to download.
    ///
    /// With an explicit `format_id`, returns that exact format. Otherwise prefers
    /// the best single progressive (video+audio) format; if none exists — direct
    /// media files and DASH video/audio-split sources — it falls back to the best
    /// available format rather than failing, so the engine/yt-dlp path can still
    /// fetch it. (Returning an error here is what previously stopped the GUI from
    /// ever starting a download.)
    fn select_format(video_info: &VideoInfo, format_id: Option<String>) -> Result<Format, String> {
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
                // No single progressive (video+audio) format. This is the common
                // case for direct media files (a lone format with no codec info)
                // and for DASH sources whose video and audio are split — failing
                // here is what stopped the GUI from ever starting a download.
                // Fall back to the best available format (by resolution) so the
                // engine / yt-dlp path can still fetch it.
                video_info
                    .formats
                    .iter()
                    .filter(|f| !f.format_id.starts_with("sb"))
                    .max_by_key(|f| f.width.unwrap_or(0) * f.height.unwrap_or(0))
                    .or_else(|| video_info.formats.first())
                    .cloned()
                    .ok_or_else(|| "No downloadable format found".to_string())
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

    async fn monitor_loop(
        qm: Arc<QueueManager>,
        sender: mpsc::Sender<BackendEvent>,
        db_manager: Arc<DatabaseManager>,
    ) {
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
                    let (status_str, completed_at, error_message) =
                        task_status_db_fields(&task.status);

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
                                file_path: Some(task.output_path.to_string_lossy().to_string()),
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

                    // Persist the transition to the `downloads` history table.
                    // Best-effort: the live queue (via `last_statuses`/`qm`)
                    // remains the runtime authority for this task's state
                    // regardless of whether this write succeeds — a failure
                    // here must never affect the download itself.
                    let record = DownloadRecord {
                        id: task.id.clone(),
                        url: task.video_info.url.clone(),
                        title: task.video_info.title.clone(),
                        output_path: task.output_path.clone(),
                        file_size: task.format.filesize,
                        status: status_str,
                        created_at: task.added_at,
                        completed_at,
                        error_message,
                    };
                    if let Err(e) = db_manager.save_download(&record).await {
                        warn!(
                            "Failed to persist download history update for {}: {}",
                            task.id, e
                        );
                    }

                    last_statuses.insert(task.id.clone(), task.status.clone());
                }

                // Send Progress only while the task is actually active — never
                // for terminal states (Completed/Failed/Cancelled), otherwise a
                // late progress event would flip a finished row back to
                // "Downloading" in the GUI.
                let active = matches!(
                    task.status,
                    TaskStatus::Downloading | TaskStatus::Queued | TaskStatus::Paused
                );
                if active {
                    if let Some(progress) = &task.progress {
                        let data = DownloadProgressData {
                            progress: progress.percentage() as f32,
                            speed: progress.speed,
                            downloaded: progress.downloaded_bytes,
                            total: progress.total_bytes,
                            eta: progress.eta.map(|d| d.as_secs()),
                        };

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_status_db_fields_sets_completed_at_only_for_terminal_states() {
        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Queued);
        assert_eq!(status, "Queued");
        assert!(completed_at.is_none());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Downloading);
        assert_eq!(status, "Downloading");
        assert!(completed_at.is_none());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Paused);
        assert_eq!(status, "Paused");
        assert!(completed_at.is_none());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Completed);
        assert_eq!(status, "Completed");
        assert!(completed_at.is_some());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Cancelled);
        assert_eq!(status, "Cancelled");
        assert!(completed_at.is_some());
        assert!(error.is_none());
    }

    #[test]
    fn task_status_db_fields_carries_the_failure_message() {
        let (status, completed_at, error) =
            task_status_db_fields(&TaskStatus::Failed("boom".to_string()));
        assert_eq!(status, "Failed");
        assert!(completed_at.is_some());
        assert_eq!(error.as_deref(), Some("boom"));
    }

    fn fmt(id: &str, vcodec: Option<&str>, acodec: Option<&str>, w: u32, h: u32) -> Format {
        Format {
            format_id: id.to_string(),
            vcodec: vcodec.map(str::to_string),
            acodec: acodec.map(str::to_string),
            width: Some(w),
            height: Some(h),
            url: format!("https://example.com/{id}"),
            ..Default::default()
        }
    }

    fn info(formats: Vec<Format>) -> VideoInfo {
        VideoInfo {
            title: "t".to_string(),
            url: "https://example.com/page".to_string(),
            formats,
            ..Default::default()
        }
    }

    #[test]
    fn explicit_format_id_is_returned() {
        let vi = info(vec![fmt("18", Some("h264"), Some("aac"), 640, 360)]);
        let f = BackendActor::select_format(&vi, Some("18".to_string())).unwrap();
        assert_eq!(f.format_id, "18");
    }

    #[test]
    fn prefers_best_progressive_format() {
        let vi = info(vec![
            fmt("low", Some("h264"), Some("aac"), 640, 360),
            fmt("high", Some("h264"), Some("aac"), 1920, 1080),
            fmt("videoonly", Some("vp9"), Some("none"), 3840, 2160),
        ]);
        let f = BackendActor::select_format(&vi, None).unwrap();
        assert_eq!(f.format_id, "high"); // best *progressive*, not the 4k video-only
    }

    #[test]
    fn direct_file_single_format_no_codecs_now_selectable() {
        // A direct .mp4 yields one format with no codec info — previously this
        // returned Err("No combined format found") and the GUI never downloaded.
        let vi = info(vec![Format {
            format_id: "0".to_string(),
            url: "https://example.com/video.mp4".to_string(),
            ..Default::default()
        }]);
        let f = BackendActor::select_format(&vi, None).expect("must pick a format, not error");
        assert_eq!(f.format_id, "0");
    }

    #[test]
    fn dash_split_falls_back_to_best_available() {
        // No single progressive format (video-only + audio-only). Must fall back
        // rather than fail.
        let vi = info(vec![
            fmt("video", Some("vp9"), Some("none"), 1920, 1080),
            fmt("audio", Some("none"), Some("mp4a"), 0, 0),
        ]);
        let f = BackendActor::select_format(&vi, None).expect("must fall back, not error");
        assert_eq!(f.format_id, "video"); // best by resolution
    }

    #[test]
    fn empty_formats_is_an_error() {
        let vi = info(vec![]);
        assert!(BackendActor::select_format(&vi, None).is_err());
    }
}
