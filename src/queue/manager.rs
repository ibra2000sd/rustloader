//! Download queue manager with concurrent download support

use crate::downloader::{DownloadEngine, DownloadProgress};
use crate::extractor::{Format, VideoInfo};
use crate::utils::error::RustloaderError;
use crate::utils::{FileOrganizer, MetadataManager, ContentType, VideoMetadata};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::time::Duration;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Download queue manager with concurrent download support
pub struct QueueManager {
    queue: Arc<Mutex<VecDeque<DownloadTask>>>,
    active_downloads: Arc<Mutex<HashMap<String, DownloadHandle>>>,
    max_concurrent: usize,
    engine: Arc<DownloadEngine>,
    file_organizer: Arc<FileOrganizer>,
    metadata_manager: Arc<MetadataManager>,
}

/// Download task
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub id: String,
    pub video_info: VideoInfo,
    pub output_path: PathBuf,
    pub format: Format,
    pub status: TaskStatus,
    pub progress: Option<DownloadProgress>,
    pub added_at: DateTime<Utc>,
}

/// Task status
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

/// Download handle for managing active downloads
struct DownloadHandle {
    task_id: String,
    join_handle: JoinHandle<()>,
    progress_handle: JoinHandle<()>,
    cancel_tx: mpsc::Sender<()>,
    // snapshot of the task for monitoring
    task: DownloadTask,
}

impl QueueManager {
    /// Create new queue manager
    pub fn new(
        max_concurrent: usize,
        engine: DownloadEngine,
        file_organizer: FileOrganizer,
        metadata_manager: MetadataManager,
    ) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent,
            engine: Arc::new(engine),
            file_organizer: Arc::new(file_organizer),
            metadata_manager: Arc::new(metadata_manager),
        }
    }

    /// Add task to queue
    pub async fn add_task(&self, task: DownloadTask) -> Result<String> {
        let task_id = task.id.clone();

        eprintln!("📥 [QUEUE] add_task called for: {}", task_id);

        // Add to queue
        {
            let mut queue = self.queue.lock().await;
            queue.push_back(task);
            eprintln!("📋 [QUEUE] Task added, queue size: {}", queue.len());
        }

        info!("Added task {} to queue", task_id);
        eprintln!("📋 [QUEUE] Task added, waiting for persistent loop to pick it up...");

        // DO NOT call process_queue() here.
        // If we call it here, it runs in the caller's runtime context.
        // The GUI creates a temporary runtime for the add_task call, which is dropped immediately.
        // If we spawn the download task here, it dies with the temporary runtime.
        // We must let the persistent start() loop (running on the main app runtime) pick it up.

        eprintln!("✅ [QUEUE] add_task completed for: {}", task_id);

        Ok(task_id)
    }

    /// Start processing queue
    pub async fn start(&self) {
        info!("Starting queue processing (persistent loop)");

        loop {
            // Process available queued tasks
            self.process_queue().await;

            // Short sleep between iterations to avoid busy-loop
            tokio::time::sleep(Duration::from_millis(500)).await;

            // If no work present, sleep a bit longer
            let has_work = {
                let queue = self.queue.lock().await;
                let active = self.active_downloads.lock().await;
                !queue.is_empty() || !active.is_empty()
            };

            if !has_work {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    /// Pause specific task
    pub async fn pause_task(&self, task_id: &str) -> Result<()> {
        // Check if task is in active downloads
        let mut active = self.active_downloads.lock().await;

        if let Some(handle) = active.get(task_id) {
            // Send cancel signal
            if let Err(e) = handle.cancel_tx.send(()).await {
                warn!("Failed to send pause signal to task {}: {}", task_id, e);
                return Err(RustloaderError::OperationFailed(format!("Failed to pause task: {}", e)).into());
            }

            // Update task status
            self.update_task_status(task_id, TaskStatus::Paused).await?;

            info!("Paused task {}", task_id);
            return Ok(());
        }

        // Check if task is in queue
        let mut queue = self.queue.lock().await;
        for task in queue.iter_mut() {
            if task.id == task_id {
                task.status = TaskStatus::Paused;
                info!("Paused queued task {}", task_id);
                return Ok(());
            }
        }

        Err(RustloaderError::TaskNotFound(task_id.to_string()).into())
    }

    /// Resume specific task
    pub async fn resume_task(&self, task_id: &str) -> Result<()> {
        // Check if task is in queue and paused
        let mut queue = self.queue.lock().await;
        for task in queue.iter_mut() {
            if task.id == task_id && task.status == TaskStatus::Paused {
                task.status = TaskStatus::Queued;
                info!("Resumed task {}", task_id);
                return Ok(());
            }
        }

        // Task not found or not paused
        Err(RustloaderError::TaskNotFound(task_id.to_string()).into())
    }

    /// Cancel task
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        // Check if task is in active downloads
        let mut active = self.active_downloads.lock().await;

        if let Some(handle) = active.remove(task_id) {
            // Send cancel signal
            if let Err(e) = handle.cancel_tx.send(()).await {
                warn!("Failed to send cancel signal to task {}: {}", task_id, e);
            }

            // Abort the task
            handle.join_handle.abort();
            // Also abort the progress receiver task so it doesn't keep running
            handle.progress_handle.abort();

            // Update task status
            self.update_task_status(task_id, TaskStatus::Cancelled).await?;

            info!("Cancelled task {}", task_id);
            return Ok(());
        }

        // Check if task is in queue
        let mut queue = self.queue.lock().await;
        for i in 0..queue.len() {
            if queue[i].id == task_id {
                queue[i].status = TaskStatus::Cancelled;
                info!("Cancelled queued task {}", task_id);
                return Ok(());
            }
        }

        Err(RustloaderError::TaskNotFound(task_id.to_string()).into())
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> Vec<DownloadTask> {
        let mut tasks = Vec::new();

        // Add queued tasks
        {
            let queue = self.queue.lock().await;
            for t in queue.iter() {
                tasks.push(t.clone());
            }
        }

        // Add active tasks
        {
            let active = self.active_downloads.lock().await;
            for (_k, handle) in active.iter() {
                tasks.push(handle.task.clone());
            }
        }

        tasks
    }

    /// Clear completed tasks
    pub async fn clear_completed(&self) -> Result<()> {
        let mut queue = self.queue.lock().await;
        queue.retain(|task| task.status != TaskStatus::Completed);
        
        let mut active = self.active_downloads.lock().await;
        active.retain(|_, handle| handle.task.status != TaskStatus::Completed);

        info!("Cleared completed tasks from queue and active downloads");
        Ok(())
    }

    /// Remove a specific task (for Remove button on individual tasks)
    pub async fn remove_task(&self, task_id: &str) -> Result<()> {
        // Remove from queue
        {
            let mut queue = self.queue.lock().await;
            queue.retain(|task| task.id != task_id);
        }

        // Remove from active downloads
        {
            let mut active = self.active_downloads.lock().await;
            active.remove(task_id);
        }

        info!("Removed task {} from queue and active downloads", task_id);
        Ok(())
    }

    /// Process the queue
    async fn process_queue(&self) {
        eprintln!("⚙️  [QUEUE] process_queue started");

        // Check if we can start more downloads
        let active_count = {
            let active = self.active_downloads.lock().await;
            active.len()
        };
        eprintln!("   - Active downloads: {}", active_count);
        eprintln!("   - Max concurrent: {}", self.max_concurrent);

        if active_count >= self.max_concurrent {
            eprintln!("⚠️  [QUEUE] Max concurrent reached, skipping");
            return;
        }

        // Get tasks to process
        let tasks_to_process = {
            let mut queue = self.queue.lock().await;
            eprintln!("   - Queue size: {}", queue.len());
            let mut tasks = Vec::new();

            while tasks.len() < self.max_concurrent - active_count {
                if let Some(task) = queue.front() {
                    if task.status == TaskStatus::Queued {
                        // Get the task and remove it from the front
                        let task = queue.pop_front().unwrap();
                        tasks.push(task);
                    } else {
                        // Skip non-queued tasks
                        queue.pop_front();
                    }
                } else {
                    break;
                }
            }

            tasks
        };

        if tasks_to_process.is_empty() {
            eprintln!("⚠️  [QUEUE] No queued tasks to start");
        }

        // Process each task
        for task in tasks_to_process {
            eprintln!("🎯 [QUEUE] Got task from queue: {}", task.id);
            eprintln!("🚀 [QUEUE] Starting download for: {}", task.id);
            self.start_download(task).await;
            eprintln!("✅ [QUEUE] start_download completed");
        }
    }

    /// Start downloading a task
    async fn start_download(&self, mut task: DownloadTask) {
        eprintln!("\n🎬 [QUEUE] ========== start_download CALLED ==========");
        eprintln!("   - Task ID: {}", task.id);
        eprintln!("   - Title: {}", task.video_info.title);
        eprintln!("   - Output: {:?}", task.output_path);
        eprintln!("   - Format: {}", task.format.format_id);
        eprintln!("   - Format URL: {}", task.format.url);
        
        let task_id = task.id.clone();
        let output_path = task.output_path.clone();
        let url = task.format.url.clone();

        eprintln!("💾 [DOWNLOAD] start_download called for: {}", task_id);
        eprintln!("   - URL: {}", url);
        eprintln!("   - Output: {:?}", output_path);

        // Create channels for progress updates and cancellation
        let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        eprintln!("   - Created progress channel");
        eprintln!("   - Starting download engine...");

        // Update task status
        task.status = TaskStatus::Downloading;
        self.update_task_in_queue(task.clone()).await;

        // Clone engine for the task
        let engine = Arc::clone(&self.engine);
        let active_downloads = Arc::clone(&self.active_downloads);
        let queue = Arc::clone(&self.queue);
        // Clone for progress handler to update active_downloads snapshot
        let active_downloads_clone = Arc::clone(&self.active_downloads);
        
        // Clone file organizer and metadata manager for the spawned task
        let file_organizer = Arc::clone(&self.file_organizer);
        let metadata_manager = Arc::clone(&self.metadata_manager);

        // Clone a snapshot of the task to keep in the active map
        let task_for_handle = task.clone();

        // Handle progress updates - spawn the receiver FIRST so the channel is ready
        // before the engine attempts to send progress messages.
        let queue_clone_for_progress = Arc::clone(&self.queue);
        let task_id_for_progress = task_id.clone();
        // snapshot of the task to use when inserting a placeholder into active_downloads
        let task_snapshot_for_progress = task_for_handle.clone();
        // Clone the active_downloads Arc into the progress handler so it can update
        // the active snapshot when progress arrives.
        let active_downloads_clone_for_progress = Arc::clone(&active_downloads_clone);
        eprintln!("📡 [PROGRESS] Spawning progress receiver for: {}", task_id_for_progress);
        let progress_handler = tokio::spawn(async move {
            eprintln!("📡 [PROGRESS] Progress receiver started for: {}", task_id_for_progress);
            loop {
                match progress_rx.recv().await {
                    Some(progress) => {
                        eprintln!("🔁 [PROGRESS] Received for {}: {:.2}%", task_id_for_progress, progress.percentage() as f32);

                        // Update task progress in the queued snapshot
                        let mut queue = queue_clone_for_progress.lock().await;
                        for t in queue.iter_mut() {
                            if t.id == task_id_for_progress {
                                t.progress = Some(progress.clone());
                                break;
                            }
                        }
                        drop(queue); // release queue lock immediately

                        // CRITICAL: Also update the active_downloads snapshot so the monitor
                        // (which reads active downloads) will see progress updates.
                        let mut active = active_downloads_clone_for_progress.lock().await;
                        if let Some(handle) = active.get_mut(&task_id_for_progress) {
                            handle.task.progress = Some(progress.clone());
                            eprintln!("✅ [PROGRESS] Updated active_downloads for: {}", task_id_for_progress);
                        } else {
                            // If no active handle exists yet (race), insert a placeholder
                            // so the monitor can see the task snapshot with progress.
                            let (dummy_cancel_tx, _dummy_cancel_rx) = mpsc::channel::<()>(1);
                            let dummy_join = tokio::spawn(async {});
                            let dummy_progress_handle = tokio::spawn(async {});
                            active.insert(
                                task_id_for_progress.clone(),
                                DownloadHandle {
                                    task_id: task_id_for_progress.clone(),
                                    join_handle: dummy_join,
                                    progress_handle: dummy_progress_handle,
                                    cancel_tx: dummy_cancel_tx,
                                    task: task_snapshot_for_progress.clone(),
                                },
                            );
                            if let Some(h) = active.get_mut(&task_id_for_progress) {
                                h.task.progress = Some(progress.clone());
                            }
                            eprintln!("✅ [PROGRESS] Inserted placeholder active_downloads for: {}", task_id_for_progress);
                        }
                        // active lock dropped at end of scope
                    }
                    None => {
                        eprintln!("📡 [PROGRESS] progress_rx closed (None) for: {}", task_id_for_progress);
                        break;
                    }
                }
            }
            eprintln!("📡 [PROGRESS] Progress receiver ended for: {}", task_id_for_progress);
        });

        eprintln!("   - Progress receiver spawned and waiting");

        // Start download task (task moved into the spawned future)
        let task_id_for_spawn = task_id.clone();

        eprintln!("🔧 [SPAWN] About to spawn download engine for: {}", task_id_for_spawn);

        // WATCHDOG: spawn a short monitor that will print if engine logs don't appear
        {
            let task_id_wd = task_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                eprintln!("⏰ [WATCHDOG] 2 seconds passed for task: {}", task_id_wd);
                eprintln!("   - If no engine logs above, the engine task is blocked or not being polled");
            });
        }

        // Prepare a clone of the task id for the spawned closure so we don't move the
        // original `task_id_for_spawn` (we need it later for a debug print).
        let task_id_for_closure = task_id_for_spawn.clone();

        let join_handle = tokio::spawn(async move {
            eprintln!("👋 [SPAWN] Inside spawned task! Task: {}", task_id_for_closure);
            eprintln!("👋 [SPAWN] About to call engine.download()");

            let mut cancelled = false;

            // Create a future that completes when either the download finishes or is cancelled
            let download_task = engine.download(&url, &output_path, progress_tx.clone());
            let cancel_task = cancel_rx.recv();

            tokio::select! {
                result = download_task => {
                    match result {
                        Ok(()) => {
                            eprintln!("✅ [ENGINE] Task {} download completed successfully", task_id_for_closure);
                            
                            // Organize the downloaded file
                            eprintln!("🎯 [ORGANIZE] Starting file organization for task {}", task_id_for_closure);
                            match Self::organize_completed_file_static(
                                file_organizer.clone(),
                                metadata_manager.clone(),
                                &task,
                                &output_path,
                            ).await {
                                Ok(final_path) => {
                                    eprintln!("✅ [ORGANIZE] File organized at: {:?}", final_path);
                                    task.output_path = final_path;
                                    task.status = TaskStatus::Completed;
                                    info!("Task {} completed and organized successfully", task_id_for_closure);
                                }
                                Err(e) => {
                                    eprintln!("⚠️  [ORGANIZE] Organization failed (non-fatal): {}", e);
                                    // Don't fail the task, file is still downloaded
                                    task.status = TaskStatus::Completed;
                                    warn!("Task {} completed but organization failed: {}", task_id_for_closure, e);
                                }
                            }
                        }
                        Err(e) => {
                            // Update task status to failed
                            task.status = TaskStatus::Failed(e.to_string());
                            eprintln!("❌ [ENGINE] Task {} failed: {}", task_id_for_closure, e);
                            error!("Task {} failed: {}", task_id_for_closure, e);
                        }
                    }
                }
                _ = cancel_task => {
                    // Task was cancelled
                    cancelled = true;
                    task.status = TaskStatus::Cancelled;
                    eprintln!("🛑 [ENGINE] Task {} was cancelled", task_id_for_closure);
                    info!("Task {} was cancelled", task_id_for_closure);
                }
            }

            // Update task in queue
            {
                let mut q = queue.lock().await;
                for t in q.iter_mut() {
                    if t.id == task_id_for_closure {
                        t.status = task.status.clone();
                        t.output_path = task.output_path.clone();
                        break;
                    }
                }
            }

            // Update task in active downloads (keep completed tasks for GUI display)
            // Only remove if cancelled or failed
            {
                let mut active = active_downloads.lock().await;
                if let Some(handle) = active.get_mut(&task_id_for_closure) {
                    handle.task.status = task.status.clone();
                    handle.task.output_path = task.output_path.clone();
                    
                    // Only remove if cancelled - keep completed tasks for GUI
                    if cancelled {
                        active.remove(&task_id_for_closure);
                    }
                }
            }

            // If not cancelled, continue processing the queue
            if !cancelled {
                drop(active_downloads);
                drop(queue);
            }
        });

        eprintln!("✅ [SPAWN] Spawned download engine task, join_handle created for: {}", task_id_for_spawn);

        // Add to active downloads
        {
            let mut active = self.active_downloads.lock().await;

            // Preserve any progress that might have been recorded by the progress
            // handler earlier (race) by copying it into the task snapshot we insert.
            let existing_progress = if let Some(existing) = active.get(&task_id) {
                existing.task.progress.clone()
            } else {
                None
            };

            let mut task_for_insert = task_for_handle.clone();
            task_for_insert.progress = existing_progress;

            active.insert(
                task_id.clone(),
                DownloadHandle {
                    task_id: task_id.clone(),
                    join_handle,
                    progress_handle: progress_handler,
                    cancel_tx,
                    task: task_for_insert,
                },
            );
        }

        eprintln!("✅ [DOWNLOAD] Task spawned and added to active_downloads: {}", task_id);

        // We keep the progress_handler running alongside the engine.
        // Note: if you want to monitor or join the handler, you can store the handle.
        info!("Started download for task {}", task_id);
    }

    /// Update task status in queue
    async fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<()> {
        // First try to update active downloads
        {
            let mut active = self.active_downloads.lock().await;
            if let Some(handle) = active.get_mut(task_id) {
                handle.task.status = status;
                return Ok(());
            }
        }

        // Then try queued tasks
        let mut queue = self.queue.lock().await;
        for task in queue.iter_mut() {
            if task.id == task_id {
                task.status = status;
                return Ok(());
            }
        }

        Err(RustloaderError::TaskNotFound(task_id.to_string()).into())
    }

    /// Update entire task in queue
    async fn update_task_in_queue(&self, task: DownloadTask) {
        // Update queued task if present
        let mut found = false;
        {
            let mut queue = self.queue.lock().await;
            for t in queue.iter_mut() {
                if t.id == task.id {
                    *t = task.clone();
                    found = true;
                    break;
                }
            }
        }

        // If not found in queue, update active snapshot
        if !found {
            let mut active = self.active_downloads.lock().await;
            if let Some(handle) = active.get_mut(&task.id) {
                handle.task = task;
            }
        }
    }
    
    /// Organize completed file (static version for use in spawned tasks)
    async fn organize_completed_file_static(
        file_organizer: Arc<FileOrganizer>,
        metadata_manager: Arc<MetadataManager>,
        task: &DownloadTask,
        downloaded_file_path: &std::path::Path,
    ) -> Result<PathBuf> {
        eprintln!("🎯 [ORGANIZE] Starting file organization...");
        eprintln!("   - Downloaded file: {:?}", downloaded_file_path);
        eprintln!("   - Video: {}", task.video_info.title);
        eprintln!("   - Format: {}", task.format.format_id);
        
        // Check if file exists
        if !downloaded_file_path.exists() {
            eprintln!("❌ [ORGANIZE] Downloaded file not found!");
            return Err(anyhow::anyhow!("Downloaded file not found: {:?}", downloaded_file_path));
        }
        
        // Determine quality string from format
        let quality = Self::determine_quality_string_static(&task.format);
        eprintln!("   - Detected quality: {}", quality);
        
        // Determine content type (default to Video for now)
        let content_type = ContentType::Video;
        
        // Organize the file (move to proper location)
        let final_path = file_organizer
            .organize_file(downloaded_file_path, &task.video_info, &quality, &content_type)
            .await
            .map_err(|e| {
                eprintln!("❌ [ORGANIZE] Failed to organize file: {}", e);
                anyhow::anyhow!("Failed to organize file: {}", e)
            })?;
        
        eprintln!("✅ [ORGANIZE] File organized at: {:?}", final_path);
        
        // Get file size
        let file_size = tokio::fs::metadata(&final_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        
        // Extract video ID
        let video_id = FileOrganizer::extract_video_id(&task.video_info.url)
            .unwrap_or(&task.video_info.id)
            .to_string();
        
        // Create and save metadata
        let metadata = VideoMetadata {
            video_id: video_id.clone(),
            title: task.video_info.title.clone(),
            url: task.video_info.url.clone(),
            source_platform: FileOrganizer::detect_source_platform(&task.video_info.url),
            duration: task.video_info.duration.map(|d| d as f64),
            resolution: quality.clone(),
            format: task.format.ext.clone(),
            file_size,
            download_date: chrono::Utc::now(),
            channel: task.video_info.uploader.clone(),
            uploader: task.video_info.uploader.clone(),
            description: task.video_info.description.clone(),
            thumbnail_url: task.video_info.thumbnail.clone(),
            quality_tier: format!("{:?}", FileOrganizer::determine_quality_tier(&quality)),
            content_type: format!("{:?}", content_type),
            tags: Vec::new(),
            favorite: false,
            watch_count: 0,
            last_accessed: chrono::Utc::now(),
        };
        
        match metadata_manager.save_metadata(&video_id, &metadata).await {
            Ok(_) => {
                eprintln!("💾 [ORGANIZE] Metadata saved successfully");
            }
            Err(e) => {
                eprintln!("⚠️  [ORGANIZE] Failed to save metadata (non-fatal): {}", e);
                // Don't fail the whole operation if metadata save fails
            }
        }
        
        eprintln!("🎉 [ORGANIZE] Organization complete!");
        Ok(final_path)
    }
    
    /// Extract quality string from format (static version)
    fn determine_quality_string_static(format: &Format) -> String {
        // Try height first
        if let Some(height) = format.height {
            return format!("{}p", height);
        }
        
        // Try format_note
        if let Some(ref note) = format.format_note {
            if !note.is_empty() {
                return note.clone();
            }
        }
        
        // Try resolution string
        if let Some(ref res) = format.resolution {
            if !res.is_empty() && res != "audio only" {
                return res.clone();
            }
        }
        
        // Try abr for audio
        if let Some(abr) = format.abr {
            return format!("{}kbps", abr as u32);
        }
        
        // Fallback
        "unknown".to_string()
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Queued
    }
}
            
        
    


impl DownloadTask {
    /// Create a new download task
    pub fn new(
        video_info: VideoInfo,
        format: Format,
        output_path: PathBuf,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();

        Self {
            id,
            video_info,
            output_path,
            format,
            status: TaskStatus::Queued,
            progress: None,
            added_at: Utc::now(),
        }
    }
}
