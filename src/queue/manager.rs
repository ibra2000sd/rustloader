//! Download queue manager with concurrent download support
#![allow(
    dead_code,
    unused_variables,
    unused_assignments,
    unused_imports,
    unused_mut
)]

use crate::downloader::{DownloadEngine, DownloadProgress};
use super::{QueueEvent, EventLog};
use crate::extractor::{Format, VideoInfo};
use crate::utils::error::RustloaderError;
use crate::utils::{ContentType, FileOrganizer, MetadataManager, VideoMetadata};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
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
    event_log: Arc<EventLog>,
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
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TaskStatus {
    #[default]
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
        event_log: Arc<EventLog>,
    ) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent,
            engine: Arc::new(engine),
            file_organizer: Arc::new(file_organizer),
            metadata_manager: Arc::new(metadata_manager),
            event_log,
        }
    }

    /// Rehydrate queue state from event log
    pub async fn rehydrate(&self) -> Result<()> {
        info!("Rehydrating queue state from event log...");
        let events = self.event_log.read_events().await?;
        
        // Reconstruct state
        // We use a map to track the latest state of each task
        let mut tasks: HashMap<String, DownloadTask> = HashMap::new();

        for event in events {
            match event {
                QueueEvent::TaskAdded { task_id, video_info, format, output_path, timestamp } => {
                    // Create task with fully restored format
                    tasks.insert(task_id.clone(), DownloadTask {
                        id: task_id,
                        video_info,
                        output_path,
                        format, // Use the persisted format
                        status: TaskStatus::Queued,
                        progress: None,
                        added_at: timestamp,
                    });
                }
                QueueEvent::TaskStarted { task_id, .. } => {
                     // If it was started, it might have been interrupted.
                     // On restart, we should treat it as 'Queued' to retry, or 'Paused' to let user decide?
                     // Let's set to Paused so we don't auto-blast downloads on startup.
                     if let Some(task) = tasks.get_mut(&task_id) {
                         task.status = TaskStatus::Paused;
                     }
                }
                QueueEvent::TaskPaused { task_id, .. } => {
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = TaskStatus::Paused;
                    }
                }
                QueueEvent::TaskResumed { task_id, .. } => {
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = TaskStatus::Paused; // See TaskStarted logic
                    }
                }
                QueueEvent::TaskCompleted { task_id, output_path, .. } => {
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = TaskStatus::Completed;
                        task.output_path = output_path;
                    }
                }
                QueueEvent::TaskFailed { task_id, error, .. } => {
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = TaskStatus::Failed(error);
                    }
                }
                QueueEvent::TaskRemoved { task_id, .. } => {
                    tasks.remove(&task_id);
                }
            }
        }

        // Populate the live queue
        let mut queue = self.queue.lock().await;
        // Clear existing (though mostly empty on startup)
        queue.clear();

        // Sort by added_at to maintain order? 
        // HashMap iteration is arbitrary.
        let mut sorted_tasks: Vec<DownloadTask> = tasks.into_values().collect();
        sorted_tasks.sort_by_key(|t| t.added_at);

        for task in sorted_tasks {
            // Only add tasks that are NOT completed/active?
            // Actually, we want to show completed tasks too if they persist in UI.
            // But if they are completed, they might just clutter.
            // Let's load everything for now, but `QueueManager` usually processes Queued tasks.
            queue.push_back(task);
        }

        info!("Rehydration complete. Loaded {} tasks.", queue.len());
        Ok(())
    }

    /// Add task to queue
    pub async fn add_task(&self, task: DownloadTask) -> Result<String> {
        let task_id = task.id.clone();

        debug!("📥 [QUEUE] add_task called for: {}", task_id);

        // Capture fields for logging
        let log_video_info = task.video_info.clone();
        let log_format = task.format.clone();
        let log_output_path = task.output_path.clone();

        // Add to queue
        {
            let mut queue = self.queue.lock().await;
            queue.push_back(task);
            debug!("📋 [QUEUE] Task added, queue size: {}", queue.len());
        }

        info!("Added task {} to queue", task_id);
        debug!("📋 [QUEUE] Task added, waiting for persistent loop to pick it up...");

        // DO NOT call process_queue() here.
        // If we call it here, it runs in the caller's runtime context.
        // The GUI creates a temporary runtime for the add_task call, which is dropped immediately.
        // If we spawn the download task here, it dies with the temporary runtime.
        // We must let the persistent start() loop (running on the main app runtime) pick it up.

        debug!("✅ [QUEUE] add_task completed for: {}", task_id);

        debug!("✅ [QUEUE] add_task completed for: {}", task_id);

        debug!("✅ [QUEUE] add_task completed for: {}", task_id);

        // LOG EVENT
        if let Err(e) = self.event_log.log(QueueEvent::TaskAdded {
            task_id: task_id.clone(),
            video_info: log_video_info,
            format: log_format,
            output_path: log_output_path,
            timestamp: Utc::now(),
        }).await {
            error!("Failed to log TaskAdded event: {}", e);
        }

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
        // LOCKING HIERARCHY: queue (Level 2) -> active (Level 1)
        // We need to update status in queue first (or at least hold the lock),
        // then remove from active.
        
        let mut queue = self.queue.lock().await;

        // 1. Find and update status in Master Store
        let task_opt = queue.iter_mut().find(|t| t.id == task_id);
        
        if let Some(task) = task_opt {
            let previous_status = task.status.clone();
            task.status = TaskStatus::Paused;
            info!("Paused task {} (Status: {:?} -> Paused)", task_id, previous_status);
            
            // LOG EVENT
            let _ = self.event_log.log(QueueEvent::TaskPaused { 
                task_id: task_id.to_string(), 
                timestamp: Utc::now() 
            }).await;
            
            // 2. If it was Downloading, we must cancel the active handle
            if previous_status == TaskStatus::Downloading {
               // We must drop queue lock before acquiring active lock?
               // The Hierarchy Rule says: Lock queue THEN active.
               // So we can hold queue lock while acquiring active.
               
               let mut active = self.active_downloads.lock().await;
               if let Some(handle) = active.remove(task_id) {
                   // Send cancel signal
                    if let Err(e) = handle.cancel_tx.send(()).await {
                        warn!("Failed to send pause signal to task {}: {}", task_id, e);
                    }
                    // Abort
                    handle.join_handle.abort();
                    handle.progress_handle.abort();
               }
            }
            
            return Ok(());
        }

        Err(RustloaderError::TaskNotFound(task_id.to_string()).into())
    }

    /// Resume specific task
    pub async fn resume_task(&self, task_id: &str) -> Result<()> {
        debug!("🔄 [RESUME] Attempting to resume task: {}", task_id);

        let mut queue = self.queue.lock().await;
        
        if let Some(task) = queue.iter_mut().find(|t| t.id == task_id) {
             // Resume logic: Just set to Queued
             if task.status == TaskStatus::Paused || task.status == TaskStatus::Failed("".to_string()) || matches!(task.status, TaskStatus::Failed(_)) {
                 task.status = TaskStatus::Queued;
                 info!("✅ [RESUME] Task {} set to Queued", task_id);
                 
                 // LOG EVENT
                 let _ = self.event_log.log(QueueEvent::TaskResumed { 
                     task_id: task_id.to_string(), 
                     timestamp: Utc::now() 
                 }).await;
                 
                 // Drop lock before triggering process_queue (cleaner, though not strictly required if process_queue handles its own locks correctly)
             } else {
                 return Err(RustloaderError::OperationFailed(format!(
                     "Task {} is not in a resumable state (status: {:?})",
                     task_id, task.status
                 )).into());
             }
        } else {
            return Err(RustloaderError::TaskNotFound(task_id.to_string()).into());
        }
        
        drop(queue);
        
        // Trigger scheduler to see if it can pick it up immediately
        self.process_queue().await;
        
        Ok(())
    }

    /// Cancel task
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        // LOCKING HIERARCHY: queue (Level 2) -> active (Level 1)
        let mut queue = self.queue.lock().await;

        // 1. Update Status in Queue
        let task_opt = queue.iter_mut().find(|t| t.id == task_id);
    
        if let Some(task) = task_opt {
             let was_downloading = task.status == TaskStatus::Downloading;
             task.status = TaskStatus::Cancelled;
             
             // LOG EVENT
             self.log_cancellation(task_id).await;
             
             // 2. If active, cancel engine
             if was_downloading {
                 let mut active = self.active_downloads.lock().await;
                 if let Some(handle) = active.remove(task_id) {
                    if let Err(e) = handle.cancel_tx.send(()).await {
                        warn!("Failed to send cancel signal to task {}: {}", task_id, e);
                    }
                    handle.join_handle.abort();
                    handle.progress_handle.abort();
                 }
             }
             
             info!("Cancelled task {}", task_id);
             return Ok(());
        }

        Err(RustloaderError::TaskNotFound(task_id.to_string()).into())
    }

    /// Log cancellation (Helper called by cancel_task)
    async fn log_cancellation(&self, task_id: &str) {
        let _ = self.event_log.log(QueueEvent::TaskRemoved { 
            task_id: task_id.to_string(), 
            timestamp: Utc::now() 
        }).await;
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> Vec<DownloadTask> {
        let queue = self.queue.lock().await;
        queue.iter().cloned().collect()
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

    /// Resume all paused and failed tasks
    pub async fn resume_all(&self) -> Result<()> {
        info!("🔄 [RESUME ALL] Resuming all paused and failed tasks");
        
        {
            let mut queue = self.queue.lock().await;
            for task in queue.iter_mut() {
                if task.status == TaskStatus::Paused || matches!(task.status, TaskStatus::Failed(_)) {
                    task.status = TaskStatus::Queued;
                    info!("✅ [RESUME ALL] Task {} set to Queued", task.id);
                    
                    // Log event
                    let _ = self.event_log.log(QueueEvent::TaskResumed { 
                        task_id: task.id.clone(), 
                        timestamp: Utc::now() 
                    }).await;
                }
            }
        }
        
        // Trigger scheduler once after all tasks are set to Queued
        self.process_queue().await;
        
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
        
        let _ = self.event_log.log(QueueEvent::TaskRemoved { 
            task_id: task_id.to_string(), 
            timestamp: Utc::now() 
        }).await;

        Ok(())
    }

    /// Process the queue
    /// Process the queue
    ///
    /// This is the heartbeat of the scheduler. It scans the queue for pending tasks
    /// and starts them if slots are available.
    async fn process_queue(&self) {
        debug!("⚙️  [QUEUE] process_queue heartbeat");

        // v0.5.1 ATOMIC PRE-REGISTRATION
        // Lock BOTH queue and active_downloads to ensure atomic state transitions.
        // This eliminates the race window where a task is Downloading but not in active.
        let mut queue = self.queue.lock().await;
        let mut active = self.active_downloads.lock().await;

        // Zombie Defense: Check for tasks that are Downloading but have no active handle
        // Since we now hold both locks, this check is safe.
        for task in queue.iter_mut() {
            if task.status == TaskStatus::Downloading {
                if !active.contains_key(&task.id) {
                    // This should never happen with atomic pre-registration
                    // If it does, it's a critical bug - log and fail the task
                    error!("🧟 [ZOMBIE] CRITICAL: Task {} is Downloading but not in active_downloads. This indicates a logic error.", task.id);
                    task.status = TaskStatus::Failed("Internal error: task lost (zombie)".to_string());
                }
            }
        }

        // Count active downloads from state
        let active_count = queue.iter()
            .filter(|t| t.status == TaskStatus::Downloading)
            .count();
        
        debug!("   - Active downloads (from state): {}", active_count);
        debug!("   - Max concurrent: {}", self.max_concurrent);

        if active_count >= self.max_concurrent {
            debug!("⚠️  [QUEUE] Max concurrent reached, skipping");
            return;
        }

        // Find candidates and ATOMICALLY pre-register them
        let mut tasks_to_start = Vec::new();
        let slots_available = self.max_concurrent - active_count;
        let mut started_count = 0;
        
        for task in queue.iter_mut() {
            if started_count >= slots_available {
                break;
            }
            
            if task.status == TaskStatus::Queued {
                // ATOMIC PRE-REGISTRATION:
                // Step 1: Create placeholder cancellation channel
                let (cancel_tx, _cancel_rx) = mpsc::channel::<()>(1);
                
                // Step 2: Insert placeholder into active_downloads FIRST
                let placeholder_handle = DownloadHandle {
                    task_id: task.id.clone(),
                    join_handle: tokio::spawn(async {}), // Dummy, will be replaced
                    progress_handle: tokio::spawn(async {}), // Dummy, will be replaced
                    cancel_tx,
                    task: task.clone(),
                };
                active.insert(task.id.clone(), placeholder_handle);
                
                // Step 3: NOW set status to Downloading (safe because already in active)
                task.status = TaskStatus::Downloading;
                info!("🚀 [SCHEDULER] Atomically reserved slot for task: {}", task.id);
                
                tasks_to_start.push(task.clone());
                started_count += 1;
            }
        }
        
        // Drop locks before spawning to avoid blocking
        drop(active);
        drop(queue);

        if tasks_to_start.is_empty() {
            return;
        }

        // Spawn Engines - they will UPDATE the existing active_downloads entry
        for task in tasks_to_start {
            info!("🚀 [QUEUE] Spawning engine for: {}", task.id);
            
            // Log Started Event
            let _ = self.event_log.log(QueueEvent::TaskStarted {
                task_id: task.id.clone(),
                timestamp: Utc::now(),
            }).await;

            self.start_download(task).await;
        }
    }

    /// Start downloading a task
    async fn start_download(&self, mut task: DownloadTask) {
        debug!("\n🎬 [QUEUE] ========== start_download CALLED ==========");
        debug!("   - Task ID: {}", task.id);
        debug!("   - Title: {}", task.video_info.title);
        debug!("   - Output: {:?}", task.output_path);
        debug!("   - Format: {}", task.format.format_id);
        debug!("   - Format URL: {}", task.format.url);

        let task_id = task.id.clone();
        let output_path = task.output_path.clone();
        let url = task.format.url.clone();

        info!("💾 [DOWNLOAD] start_download called for: {}", task_id);
        debug!("   - URL: {}", url);
        debug!("   - Output: {:?}", output_path);

        // Create channels for progress updates and cancellation
        let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        debug!("   - Created progress channel");
        debug!("   - Starting download engine...");

        // Update task status
        // Update task status - ALREADY DONE in process_queue via reservation
        // task.status = TaskStatus::Downloading;
        // self.update_task_in_queue(task.clone()).await;

        // Clone engine for the task
        let engine = Arc::clone(&self.engine);
        let active_downloads = Arc::clone(&self.active_downloads);
        let queue = Arc::clone(&self.queue);
        // Clone for progress handler to update active_downloads snapshot
        let active_downloads_clone = Arc::clone(&self.active_downloads);

        // Clone file organizer and metadata manager for the spawned task
        let file_organizer = Arc::clone(&self.file_organizer);
        let metadata_manager = Arc::clone(&self.metadata_manager);
        let event_log = Arc::clone(&self.event_log); // CLONE EVENT LOG

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
        debug!(
            "📡 [PROGRESS] Spawning progress receiver for: {}",
            task_id_for_progress
        );
        let progress_handler = tokio::spawn(async move {
            debug!(
                "📡 [PROGRESS] Progress receiver started for: {}",
                task_id_for_progress
            );
            loop {
                match progress_rx.recv().await {
                    Some(progress) => {
                        debug!(
                            "🔁 [PROGRESS] Received for {}: {:.2}%",
                            task_id_for_progress,
                            progress.percentage() as f32
                        );

                        // Update task progress in the queued snapshot
                        // LOCKING: We need to update the Master List (Queue) with progress
                        let mut queue = queue_clone_for_progress.lock().await;
                        if let Some(t) = queue.iter_mut().find(|t| t.id == task_id_for_progress) {
                             t.progress = Some(progress.clone());
                        }
                        drop(queue); // release queue lock immediately

                        // CRITICAL: Also update the active_downloads snapshot so the monitor
                        // (which reads active downloads) will see progress updates.
                        let mut active = active_downloads_clone_for_progress.lock().await;
                        if let Some(handle) = active.get_mut(&task_id_for_progress) {
                            handle.task.progress = Some(progress.clone());
                            debug!(
                                "✅ [PROGRESS] Updated active_downloads for: {}",
                                task_id_for_progress
                            );
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
                            debug!(
                                "✅ [PROGRESS] Inserted placeholder active_downloads for: {}",
                                task_id_for_progress
                            );
                        }
                        // active lock dropped at end of scope
                    }
                    None => {
                        debug!(
                            "📡 [PROGRESS] progress_rx closed (None) for: {}",
                            task_id_for_progress
                        );
                        break;
                    }
                }
            }
            debug!(
                "📡 [PROGRESS] Progress receiver ended for: {}",
                task_id_for_progress
            );
        });

        debug!("   - Progress receiver spawned and waiting");

        // Start download task (task moved into the spawned future)
        let task_id_for_spawn = task_id.clone();

        debug!(
            "🔧 [SPAWN] About to spawn download engine for: {}",
            task_id_for_spawn
        );

        // WATCHDOG: spawn a short monitor that will print if engine logs don't appear
        {
            let task_id_wd = task_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                warn!("⏰ [WATCHDOG] 2 seconds passed for task: {}", task_id_wd);
                warn!(
                    "   - If no engine logs above, the engine task is blocked or not being polled"
                );
            });
        }

        // Prepare a clone of the task id for the spawned closure so we don't move the
        // original `task_id_for_spawn` (we need it later for a debug print).
        let task_id_for_closure = task_id_for_spawn.clone();

        let join_handle = tokio::spawn(async move {
            debug!(
                "👋 [SPAWN] Inside spawned task! Task: {}",
                task_id_for_closure
            );
            debug!("👋 [SPAWN] About to call engine.download()");

            let mut cancelled = false;

            // Create a future that completes when either the download finishes or is cancelled
            let download_task = engine.download(&url, &output_path, progress_tx.clone());
            let cancel_task = cancel_rx.recv();

            tokio::select! {
                result = download_task => {
                    match result {
                        Ok(()) => {
                            info!("✅ [ENGINE] Task {} download completed successfully", task_id_for_closure);

                            // Organize the downloaded file
                            info!("🎯 [ORGANIZE] Starting file organization for task {}", task_id_for_closure);

                            // ✅ DEBUG BUG-007: Log pre-organization state
                            debug!("🔍 [ORGANIZE DEBUG] Pre-organization checks:");
                            debug!("   - File exists: {}", output_path.exists());
                            debug!("   - File path: {:?}", output_path);
                            debug!("   - File size: {} bytes",
                                     std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0));
                            debug!("   - Video title: {}", task.video_info.title);
                            debug!("   - Base dir: {:?}", file_organizer.base_dir);

                            match Self::organize_completed_file_static(
                                file_organizer.clone(),
                                metadata_manager.clone(),
                                &task,
                                &output_path,
                            ).await {
                                Ok(final_path) => {
                                    info!("✅ [ORGANIZE] File organized at: {:?}", final_path);
                                    task.output_path = final_path.clone();
                                    task.status = TaskStatus::Completed;
                                    info!("Task {} completed and organized successfully", task_id_for_closure);
                                    
                                    // LOG EVENT
                                    let _ = event_log.log(QueueEvent::TaskCompleted { 
                                        task_id: task_id_for_closure.clone(), 
                                        output_path: final_path, 
                                        timestamp: Utc::now() 
                                    }).await;
                                }
                                Err(e) => {
                                    // ✅ DEBUG BUG-007: Enhanced error logging
                                    error!("❌ [ORGANIZE] Organization failed: {}", e);
                                    error!("❌ [ORGANIZE] Error details: {:?}", e);
                                    error!("❌ [ORGANIZE] File remains at: {:?}", output_path);
                                    error!("❌ [ORGANIZE] File size: {} bytes",
                                             std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0));

                                    // Don't fail the task, file is still downloaded
                                    task.status = TaskStatus::Completed;
                                    task.output_path = output_path.clone();  // Keep original path

                                    warn!("Task {} completed but organization failed: {}. File at: {:?}",
                                          task_id_for_closure, e, output_path);
                                }
                            }
                        }
                        Err(e) => {
                            // Update task status to failed
                            task.status = TaskStatus::Failed(e.to_string());
                            error!("❌ [ENGINE] Task {} failed: {}", task_id_for_closure, e);
                            error!("Task {} failed: {}", task_id_for_closure, e);

                            // LOG EVENT
                            let _ = event_log.log(QueueEvent::TaskFailed { 
                                task_id: task_id_for_closure.clone(), 
                                error: e.to_string(), 
                                timestamp: Utc::now() 
                            }).await;
                        }
                    }
                }
                _ = cancel_task => {
                    // Task was cancelled
                    cancelled = true;
                    task.status = TaskStatus::Cancelled;
                    warn!("🛑 [ENGINE] Task {} was cancelled", task_id_for_closure);
                    info!("Task {} was cancelled", task_id_for_closure);
                }
            }

            // Update task in queue (Master Store)
            {
                let mut q = queue.lock().await;
                if let Some(t) = q.iter_mut().find(|t| t.id == task_id_for_closure) {
                    t.status = task.status.clone();
                    t.output_path = task.output_path.clone();
                }
            }

            // Update task in active downloads (for handles/cancellation only)
            // Remove if no longer downloading (Completed/Failed)
            // NOTE: The FSM says remove from active if Finished.
            {
                let mut active = active_downloads.lock().await;
                // If it's done (Completed/Failed), we should remove it from active map
                // to free up slots.
                // If we treat active_downloads purely as "Running Engines", then yes.
                
                // wait, if we remove it, we can't send cancel signals? 
                // But it's done. 
                
                if matches!(task.status, TaskStatus::Completed | TaskStatus::Failed(_)) {
                    active.remove(&task_id_for_closure);
                } else {
                     // Update snapshot just in case we need it for something
                     if let Some(handle) = active.get_mut(&task_id_for_closure) {
                        handle.task.status = task.status.clone();
                        handle.task.output_path = task.output_path.clone();
                         
                        // If cancelled externally, it might be removed already
                     }
                }
            }

            // If not cancelled, continue processing the queue
            if !cancelled {
                drop(active_downloads);
                drop(queue);
            }
        });

        debug!(
            "✅ [SPAWN] Spawned download engine task, join_handle created for: {}",
            task_id_for_spawn
        );

        // v0.5.1: UPDATE the existing active_downloads entry (pre-registered in process_queue)
        // instead of inserting new. This completes the atomic transaction.
        {
            let mut active = self.active_downloads.lock().await;

            if let Some(handle) = active.get_mut(&task_id) {
                // Update the placeholder with real handles
                handle.join_handle = join_handle;
                handle.progress_handle = progress_handler;
                handle.cancel_tx = cancel_tx;
                handle.task = task_for_handle.clone();
                debug!("✅ [DOWNLOAD] Updated active_downloads entry for: {}", task_id);
            } else {
                // This should never happen - placeholder was inserted in process_queue
                // If we get here, something is seriously wrong
                error!("❌ [DOWNLOAD] CRITICAL: Task {} not found in active_downloads! Placeholder missing.", task_id);
                
                // Rollback: Mark task as failed in queue
                let mut queue = self.queue.lock().await;
                if let Some(task_in_queue) = queue.iter_mut().find(|t| t.id == task_id) {
                    task_in_queue.status = TaskStatus::Failed("Internal error: pre-registration failed".to_string());
                }
                return;
            }
        }

        debug!(
            "✅ [DOWNLOAD] Task spawned and active_downloads updated: {}",
            task_id
        );

        info!("Started download for task {}", task_id);

        let _ = self.event_log.log(QueueEvent::TaskStarted { 
            task_id: task_id.to_string(), 
            timestamp: Utc::now() 
        }).await;
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
        debug!("🎯 [ORGANIZE] Starting file organization...");
        debug!("   - Downloaded file: {:?}", downloaded_file_path);
        debug!("   - Video: {}", task.video_info.title);
        debug!("   - Format: {}", task.format.format_id);

        // Check if file exists
        if !downloaded_file_path.exists() {
            error!("❌ [ORGANIZE] Downloaded file not found!");
            return Err(anyhow::anyhow!(
                "Downloaded file not found: {:?}",
                downloaded_file_path
            ));
        }

        // Determine quality string from format
        let quality = Self::determine_quality_string_static(&task.format);
        debug!("   - Detected quality: {}", quality);

        // Determine content type (default to Video for now)
        let content_type = ContentType::Video;

        // Organize the file (move to proper location)
        let final_path = file_organizer
            .organize_file(
                downloaded_file_path,
                &task.video_info,
                &quality,
                &content_type,
            )
            .await
            .map_err(|e| {
                error!("❌ [ORGANIZE] Failed to organize file: {}", e);
                anyhow::anyhow!("Failed to organize file: {}", e)
            })?;

        info!("✅ [ORGANIZE] File organized at: {:?}", final_path);

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

impl DownloadTask {
    /// Create a new download task
    pub fn new(video_info: VideoInfo, format: Format, output_path: PathBuf) -> Self {
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
