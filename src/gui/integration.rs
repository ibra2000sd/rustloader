//! Bridge between GUI and backend

use crate::downloader::{DownloadConfig, DownloadEngine, DownloadProgress};
use crate::extractor::{Format, VideoExtractor, VideoInfo};
use crate::queue::{DownloadTask, QueueManager, TaskStatus};
use crate::utils::config::AppSettings;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Backend bridge for communication between GUI and backend components
pub struct BackendBridge {
    extractor: Arc<VideoExtractor>,
    queue_manager: Arc<QueueManager>,
    progress_rx: mpsc::Receiver<ProgressUpdate>,
    _progress_tx: mpsc::Sender<ProgressUpdate>,
}

/// Progress update from backend to GUI
#[derive(Debug, Clone)]
pub enum ProgressUpdate {
    ExtractionComplete(VideoInfo),
    DownloadProgress {
        task_id: String,
        progress: f32,
        speed: f64,
        downloaded: u64,
        total: u64,
        eta_seconds: Option<u64>,
    },
    DownloadComplete(String),
    DownloadFailed {
        task_id: String,
        error: String,
    },
    TaskStatusChanged {
        task_id: String,
        status: TaskStatus,
    },
}

impl BackendBridge {
    /// Create new backend bridge
    pub async fn new(settings: AppSettings) -> Result<Self> {
        // Create channels for progress updates

        // Initialize extractor
        let extractor = Arc::new(VideoExtractor::new()?);

        // Initialize download engine
            eprintln!("🏗️ [BRIDGE-NEW] BackendBridge::new() called!");
            let (progress_tx, progress_rx) = mpsc::channel(100);
            eprintln!("🏗️ [BRIDGE-NEW] Created progress channel");

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

        // Initialize queue manager
        let queue_manager = Arc::new(QueueManager::new(
            settings.max_concurrent,
            engine,
        ));

        // Start queue processing
        let queue_manager_clone = Arc::clone(&queue_manager);
        let progress_tx_clone = progress_tx.clone();
        tokio::spawn(async move {
            eprintln!("🏗️ [BRIDGE-NEW] About to spawn queue processor...");
            // Call start() on the manager WITHOUT holding an outer Mutex here.
            queue_manager_clone.start().await;
        });

        // Start monitoring queue for progress updates
        let queue_manager_clone = Arc::clone(&queue_manager);
        tokio::spawn(async move {
            eprintln!("🏗️ [BRIDGE-NEW] Queue processor spawned");
            eprintln!("🚨 [MONITOR-SPAWN] Monitor task STARTED!");
            let mut last_statuses = std::collections::HashMap::new();
            eprintln!("🏗️ [BRIDGE-NEW] About to spawn monitor...");

            loop {
                eprintln!("🔄 [MONITOR-LOOP] Starting poll iteration");
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                eprintln!("🔄 [MONITOR-LOOP] After sleep, before lock");

                eprintln!("🔄 [MONITOR-LOOP] After lock, before get_all_tasks");
                let tasks = queue_manager_clone.get_all_tasks().await;

                eprintln!("🔍 [MONITOR] Polling: {} tasks", tasks.len());
                for task in tasks {
                    eprintln!("🔍 [MONITOR] Task {}: status={:?}, progress={:?}", 
                        &task.id[..8], task.status, task.progress.as_ref().map(|p| p.percentage()));
                    let last_status = last_statuses.get(&task.id).cloned().unwrap_or(TaskStatus::Queued);

                    // Check if status changed
                    if task.status != last_status {
                        if let Err(e) = progress_tx_clone.send(ProgressUpdate::TaskStatusChanged {
                            task_id: task.id.clone(),
                            status: task.status.clone(),
                        }).await {
                            warn!("Failed to send status update: {}", e);
                            break;
                        }
                    }

                    // Send progress updates
                    if let Some(progress) = &task.progress {
                        if let Err(e) = progress_tx_clone.send(ProgressUpdate::DownloadProgress {
                            task_id: task.id.clone(),
                            progress: progress.percentage() as f32,
                            speed: progress.speed,
                            downloaded: progress.downloaded_bytes,
                            total: progress.total_bytes,
                            eta_seconds: progress.eta.map(|d| d.as_secs()),
                        }).await {
                            warn!("Failed to send progress update: {}", e);
                            break;
                        }
                    }

                    last_statuses.insert(task.id.clone(), task.status);
                }

                eprintln!("🔄 [MONITOR-LOOP] End of iteration");
            }
        });

        eprintln!("🏗️ [BRIDGE-NEW] Monitor spawned");
        eprintln!("🏗️ [BRIDGE-NEW] Returning BackendBridge");
        Ok(Self {
            extractor,
            queue_manager,
            progress_rx,
            _progress_tx: progress_tx,
        })
    }

    /// Extract video information from URL
    pub async fn extract_video_info(&self, url: &str) -> Result<VideoInfo, String> {
        debug!("Extracting video info for: {}", url);

        match self.extractor.extract_info(url).await {
            Ok(info) => Ok(info),
            Err(e) => {
                error!("Failed to extract video info: {}", e);
                Err(e.to_string())
            }
        }
    }

    /// Start downloading a video
    pub async fn start_download(
        &self,
        video_info: VideoInfo,
        output_path: PathBuf,
        format_id: Option<String>,
    ) -> Result<String, String> {
        eprintln!("🚀 [BRIDGE] start_download called");
        eprintln!("   - Video: {}", video_info.title);
        eprintln!("   - Output: {:?}", output_path);

        // Get format
        let format = if let Some(id) = format_id {
            match video_info.formats.iter().find(|f| f.format_id == id) {
                Some(format) => format.clone(),
                None => return Err("Format not found".to_string()),
            }
        } else {
            // Use best format (compare quality as float; handle Option<f32>)
            video_info.formats
                .iter()
                .max_by(|a, b| {
                    let aq = a.quality.unwrap_or(0.0);
                    let bq = b.quality.unwrap_or(0.0);
                    aq.partial_cmp(&bq).unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
                .ok_or_else(|| "No formats available".to_string())?
        };

        // Get direct URL
        eprintln!("   - Selected format: {}", format.format_id);
        let direct_url = match self.extractor.get_direct_url(&video_info.url, &format.format_id).await {
            Ok(url) => {
                eprintln!("   - Direct URL obtained: {}", url);
                url
            }
            Err(e) => {
                eprintln!("❌ [BRIDGE] Failed to get direct URL: {}", e);
                return Err(format!("Failed to get direct URL: {}", e));
            }
        };

        // Create download task
        let task_id = Uuid::new_v4().to_string();
        eprintln!("   - Generated task_id: {}", task_id);
        let mut updated_format = format;
        updated_format.url = direct_url;

        let task = DownloadTask {
            id: task_id.clone(),
            video_info,
            output_path: output_path.clone(),
            format: updated_format,
            status: TaskStatus::Queued,
            progress: None,
            added_at: chrono::Utc::now(),
        };

        eprintln!("   - Task created, adding to queue...");

        // Add to queue
        if let Err(e) = self.queue_manager.add_task(task).await {
            eprintln!("❌ [BRIDGE] Failed to add task to queue: {}", e);
            error!("Failed to add task to queue: {}", e);
            return Err(e.to_string());
        }

        eprintln!("✅ [BRIDGE] Task added to queue successfully: {}", task_id);
        info!("Added download task to queue: {}", task_id);
        Ok(task_id)
    }

    /// Pause a download
    pub async fn pause_download(&self, task_id: &str) -> Result<(), String> {
        debug!("Pausing download: {}", task_id);

        if let Err(e) = self.queue_manager.pause_task(task_id).await {
            error!("Failed to pause task: {}", e);
            return Err(e.to_string());
        }

        Ok(())
    }

    /// Resume a download
    pub async fn resume_download(&self, task_id: &str) -> Result<(), String> {
        debug!("Resuming download: {}", task_id);

        if let Err(e) = self.queue_manager.resume_task(task_id).await {
            error!("Failed to resume task: {}", e);
            return Err(e.to_string());
        }

        Ok(())
    }

    /// Cancel a download
    pub async fn cancel_download(&self, task_id: &str) -> Result<(), String> {
        debug!("Cancelling download: {}", task_id);

        if let Err(e) = self.queue_manager.cancel_task(task_id).await {
            error!("Failed to cancel task: {}", e);
            return Err(e.to_string());
        }

        Ok(())
    }

    /// Get all download tasks
    pub async fn get_all_tasks(&self) -> Result<Vec<DownloadTask>, String> {
        Ok(self.queue_manager.get_all_tasks().await)
    }

    /// Clear completed tasks
    pub async fn clear_completed(&self) -> Result<(), String> {
        debug!("Clearing completed tasks");

        if let Err(e) = self.queue_manager.clear_completed().await {
            error!("Failed to clear completed tasks: {}", e);
            return Err(e.to_string());
        }

        Ok(())
    }

    /// Try to receive a progress update
    pub fn try_receive_progress(&mut self) -> Option<ProgressUpdate> {
        match self.progress_rx.try_recv() {
            Ok(update) => {
                eprintln!("🧩 [BRIDGE] try_receive_progress -> Some: {:?}", update);
                Some(update)
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                eprintln!("🧩 [BRIDGE] try_receive_progress -> None (empty queue)");
                None
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                eprintln!("🧩 [BRIDGE] try_receive_progress -> Err: Disconnected");
                None
            }
        }
    }
}
