//! Bridge between GUI and backend

use crate::downloader::{DownloadConfig, DownloadEngine};
use crate::extractor::{Format, VideoExtractor, VideoInfo};
use crate::queue::{DownloadTask, QueueManager, TaskStatus};
use crate::utils::config::AppSettings;
use crate::utils::{FileOrganizer, MetadataManager, OrganizationSettings, VideoMetadata, ContentType};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::Utc;

/// Backend bridge for communication between GUI and backend components
pub struct BackendBridge {
    extractor: Arc<VideoExtractor>,
    queue_manager: Arc<QueueManager>,
    progress_rx: mpsc::Receiver<ProgressUpdate>,
    _progress_tx: mpsc::Sender<ProgressUpdate>,
    file_organizer: Arc<FileOrganizer>,
    metadata_manager: Arc<MetadataManager>,
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
    DownloadComplete {
        task_id: String,
        file_path: String,
    },
    DownloadFailed {
        task_id: String,
        error: String,
    },
    TaskStatusChanged {
        task_id: String,
        status: TaskStatus,
        file_path: Option<String>,
    },
}

// Implement Clone for BackendBridge to fix mutex deadlock issues
// This is safe because all fields are Arc (cheap reference counting)
impl Clone for BackendBridge {
    fn clone(&self) -> Self {
        // Note: We can't clone progress_rx (Receiver is not Clone)
        // Create a dummy channel for the clone - the original holds the real receiver
        let (_dummy_tx, dummy_rx) = mpsc::channel(100);
        
        Self {
            extractor: Arc::clone(&self.extractor),
            queue_manager: Arc::clone(&self.queue_manager),
            progress_rx: dummy_rx,
            _progress_tx: self._progress_tx.clone(),
            file_organizer: Arc::clone(&self.file_organizer),
            metadata_manager: Arc::clone(&self.metadata_manager),
        }
    }
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
        
        // Initialize file organization system
        eprintln!("🏗️ [BRIDGE-NEW] Initializing file organizer...");
        let org_settings = OrganizationSettings::default();
        let file_organizer = FileOrganizer::new(org_settings).await
            .map_err(|e| anyhow::anyhow!("Failed to initialize file organizer: {}", e))?;
        
        let metadata_manager = MetadataManager::new(&file_organizer.base_dir);
        eprintln!("✅ [BRIDGE-NEW] File organization system initialized");

        // Initialize queue manager with organizers
        let queue_manager = Arc::new(QueueManager::new(
            settings.max_concurrent,
            engine,
            file_organizer.clone(),
            metadata_manager.clone(),
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
                        let file_path = if matches!(task.status, TaskStatus::Completed) {
                            Some(task.output_path.to_string_lossy().to_string())
                        } else {
                            None
                        };
                        
                        if let Err(e) = progress_tx_clone.send(ProgressUpdate::TaskStatusChanged {
                            task_id: task.id.clone(),
                            status: task.status.clone(),
                            file_path,
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
                    } else if matches!(task.status, TaskStatus::Completed) {
                        // Synthesize a 100% progress event when completed but progress is None
                        let downloaded = tokio::fs::metadata(&task.output_path)
                            .await
                            .map(|m| m.len())
                            .unwrap_or(0);
                        if let Err(e) = progress_tx_clone.send(ProgressUpdate::DownloadProgress {
                            task_id: task.id.clone(),
                            progress: 100.0,
                            speed: 0.0,
                            downloaded,
                            total: downloaded,
                            eta_seconds: Some(0),
                        }).await {
                            warn!("Failed to send synthesized completion progress: {}", e);
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
            file_organizer: Arc::new(file_organizer),
            metadata_manager: Arc::new(metadata_manager),
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
        eprintln!("\n🚀🚀🚀 [BRIDGE] ========== start_download CALLED ==========");

        // FIX: Ensure download goes to Rustloader directory
        let filename = output_path.file_name().ok_or("Invalid output path")?;
        let downloads_dir = dirs::download_dir().ok_or("Could not find downloads directory")
            .map_err(|e| e.to_string())?;
        let rustloader_dir = downloads_dir.join("Rustloader");
        if !rustloader_dir.exists() {
            std::fs::create_dir_all(&rustloader_dir).map_err(|e| e.to_string())?;
        }
        let output_path = rustloader_dir.join(filename);

        eprintln!("   - Video: {}", video_info.title);
        eprintln!("   - Output (fixed): {:?}", output_path);
        eprintln!("   - Format ID provided: {:?}", format_id);
        eprintln!("   - Total formats available: {}", video_info.formats.len());
        eprintln!("   - File organizer enabled: Using temp directory for download");

        // Get format
        eprintln!("\n🔍 [FORMAT-SELECT] Starting format selection...");
        let format = if let Some(id) = format_id {
            eprintln!("   [FORMAT-SELECT] User specified format ID: {}", id);
            match video_info.formats.iter().find(|f| f.format_id == id) {
                Some(format) => {
                    eprintln!("   [FORMAT-SELECT] Found user-specified format: {}", id);
                    format.clone()
                }
                None => {
                    eprintln!("   [FORMAT-SELECT] ERROR: User-specified format not found: {}", id);
                    return Err("Format not found".to_string());
                }
            }
        } else {
            eprintln!("   [FORMAT-SELECT] No format specified, auto-selecting...");
            eprintln!("🔍 [FORMAT] Total formats available: {}", video_info.formats.len());
            
            // First, try to find formats with BOTH video and audio (combined formats)
            let combined_formats: Vec<_> = video_info.formats
                .iter()
                .filter(|f| {
                    // Must have video codec
                    let has_video = if let Some(ref vcodec) = f.vcodec {
                        vcodec != "none" && !vcodec.is_empty()
                    } else {
                        false
                    };
                    
                    // Must have audio codec
                    let has_audio = if let Some(ref acodec) = f.acodec {
                        acodec != "none" && !acodec.is_empty()
                    } else {
                        false
                    };
                    
                    // Exclude storyboards and images
                    let not_storyboard = !f.format_id.starts_with("sb");
                    let not_image = f.ext != "jpg" && f.ext != "png" && f.ext != "webp";
                    
                    // Must have dimensions
                    let has_dimensions = f.width.is_some() && f.height.is_some();
                    
                    let is_combined = has_video && has_audio && not_storyboard && not_image && has_dimensions;
                    
                    if is_combined {
                        eprintln!("✅ [FORMAT] Combined format: {} ({}x{}, vcodec={:?}, acodec={:?})", 
                            f.format_id, f.width.unwrap_or(0), f.height.unwrap_or(0), f.vcodec, f.acodec);
                    }
                    
                    is_combined
                })
                .collect();

            eprintln!("🔍 [FORMAT] Found {} combined (video+audio) formats", combined_formats.len());

            // If we have combined formats, use the best one
            if !combined_formats.is_empty() {
                let best_format = combined_formats
                    .iter()
                    .max_by(|a, b| {
                        let a_res = a.width.unwrap_or(0) * a.height.unwrap_or(0);
                        let b_res = b.width.unwrap_or(0) * b.height.unwrap_or(0);
                        
                        match a_res.cmp(&b_res) {
                            std::cmp::Ordering::Equal => {
                                let aq = a.quality.unwrap_or(0.0);
                                let bq = b.quality.unwrap_or(0.0);
                                aq.partial_cmp(&bq).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            other => other,
                        }
                    });
                
                match best_format {
                    Some(fmt) => {
                        eprintln!("🎯 [FORMAT] Selected combined format: {} ({}x{}, quality={:?})", 
                            fmt.format_id, fmt.width.unwrap_or(0), fmt.height.unwrap_or(0), fmt.quality);
                        (*fmt).clone()
                    }
                    None => {
                        // Fallback: No combined formats found, return error
                        eprintln!("❌ [FORMAT] No combined video+audio formats available");
                        return Err("No combined video+audio formats available. This video requires separate video and audio download (not yet supported).".to_string());
                    }
                }
            } else {
                // Fallback: No combined formats found, return error
                eprintln!("❌ [FORMAT] No combined video+audio formats available");
                return Err("No combined video+audio formats available. This video requires separate video and audio download (not yet supported).".to_string());
            }
        };

        eprintln!("\n✅ [FORMAT-SELECT] Format selection complete!");
        eprintln!("   - Selected format ID: {}", format.format_id);
        eprintln!("   - Format ext: {}", format.ext);
        eprintln!("   - Format vcodec: {:?}", format.vcodec);
        eprintln!("   - Format acodec: {:?}", format.acodec);
        eprintln!("   - Format resolution: {:?}x{:?}", format.width, format.height);

        // Get direct URL
        eprintln!("\n🌐 [URL-FETCH] Fetching direct URL for format {}...", format.format_id);
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

        // If the direct URL points to an HLS manifest (m3u8 / manifest / playlist),
        // we should hand yt-dlp the original video page URL instead of the manifest URL.
        // Passing the manifest back to yt-dlp causes it to behave unexpectedly and
        // produces no useful progress output; the engine may then report the small
        // manifest size (e.g. 50 bytes) instead of the actual media size.
        let download_url = if direct_url.contains(".m3u8")
            || direct_url.contains("/manifest")
            || direct_url.contains("playlist")
        {
            eprintln!("🔀 [URL-DETECT] HLS stream detected, will use original URL for yt-dlp");
            eprintln!("   - Manifest URL: {}", direct_url);
            eprintln!("   - Will use: {}", video_info.url);
            video_info.url.clone()
        } else {
            eprintln!("📦 [URL-DETECT] Direct download URL detected: {}", direct_url);
            direct_url.clone()
        };

        // Create download task
        let task_id = Uuid::new_v4().to_string();
        eprintln!("   - Generated task_id: {}", task_id);
        let mut updated_format = format;
        updated_format.url = download_url;

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

    /// Remove a specific task
    pub async fn remove_task(&self, task_id: &str) -> Result<(), String> {
        debug!("Removing task: {}", task_id);

        if let Err(e) = self.queue_manager.remove_task(task_id).await {
            error!("Failed to remove task {}: {}", task_id, e);
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
    
    /// Organize downloaded file and save metadata
    pub async fn organize_completed_download(
        &self,
        task_id: &str,
        temp_path: &PathBuf,
        video_info: &VideoInfo,
        format: &Format,
    ) -> Result<PathBuf, String> {
        eprintln!("🗂️  [ORGANIZE] Starting file organization for task: {}", task_id);
        
        // Determine content type (for now, assume simple video)
        let content_type = ContentType::Video;
        
        // Get quality string from format
        let quality = if let (Some(w), Some(h)) = (format.width, format.height) {
            format!("{}p", h)
        } else {
            "unknown".to_string()
        };
        
        // Organize the file
        let final_path = self.file_organizer
            .organize_file(temp_path, video_info, &quality, &content_type)
            .await
            .map_err(|e| format!("Failed to organize file: {}", e))?;
        
        eprintln!("✅ [ORGANIZE] File organized to: {:?}", final_path);
        
        // Extract video ID for metadata
        let video_id = FileOrganizer::extract_video_id(&video_info.url)
            .unwrap_or(&task_id)
            .to_string();
        
        // Get file size
        let file_size = tokio::fs::metadata(&final_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        
        // Create metadata
        let metadata = VideoMetadata {
            video_id: video_id.clone(),
            title: video_info.title.clone(),
            url: video_info.url.clone(),
            source_platform: FileOrganizer::detect_source_platform(&video_info.url),
            duration: video_info.duration.map(|d| d as f64),
            resolution: quality.clone(),
            format: format.ext.clone(),
            file_size,
            download_date: Utc::now(),
            channel: video_info.uploader.clone(),
            uploader: video_info.uploader.clone(),
            description: video_info.description.clone(),
            thumbnail_url: video_info.thumbnail.clone(),
            quality_tier: format!("{:?}", FileOrganizer::determine_quality_tier(&quality)),
            content_type: format!("{:?}", content_type),
            tags: Vec::new(),
            favorite: false,
            watch_count: 0,
            last_accessed: Utc::now(),
        };
        
        // Save metadata
        if let Err(e) = self.metadata_manager.save_metadata(&video_id, &metadata).await {
            eprintln!("⚠️  [ORGANIZE] Failed to save metadata: {}", e);
        } else {
            eprintln!("✅ [ORGANIZE] Metadata saved successfully");
        }
        
        Ok(final_path)
    }
    
    /// Get file organizer reference
    pub fn get_file_organizer(&self) -> Arc<FileOrganizer> {
        Arc::clone(&self.file_organizer)
    }
    
    /// Get metadata manager reference
    pub fn get_metadata_manager(&self) -> Arc<MetadataManager> {
        Arc::clone(&self.metadata_manager)
    }
}
