//! Download queue manager with concurrent download support

use crate::downloader::{DownloadEngine, DownloadProgress};
use crate::extractor::{Format, VideoInfo};
use crate::utils::error::RustloaderError;
use anyhow::Result;
use chrono::{DateTime, Utc};
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
    cancel_tx: mpsc::Sender<()>,
}

impl QueueManager {
    /// Create new queue manager
    pub fn new(max_concurrent: usize, engine: DownloadEngine) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent,
            engine: Arc::new(engine),
        }
    }

    /// Add task to queue
    pub async fn add_task(&self, task: DownloadTask) -> Result<String> {
        let task_id = task.id.clone();

        // Add to queue
        {
            let mut queue = self.queue.lock().await;
            queue.push_back(task);
        }

        info!("Added task {} to queue", task_id);

        // Start processing if not already running
        self.process_queue().await;

        Ok(task_id)
    }

    /// Start processing queue
    pub async fn start(&self) {
        info!("Starting queue processing");
        self.process_queue().await;
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
        let queue = self.queue.lock().await;
        queue.iter().cloned().collect()
    }

    /// Clear completed tasks
    pub async fn clear_completed(&self) -> Result<()> {
        let mut queue = self.queue.lock().await;
        queue.retain(|task| task.status != TaskStatus::Completed);

        info!("Cleared completed tasks from queue");
        Ok(())
    }

    /// Process the queue
    async fn process_queue(&self) {
        // Check if we can start more downloads
        let active_count = {
            let active = self.active_downloads.lock().await;
            active.len()
        };

        if active_count >= self.max_concurrent {
            return;
        }

        // Get tasks to process
        let tasks_to_process = {
            let mut queue = self.queue.lock().await;
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

        // Process each task
        for task in tasks_to_process {
            self.start_download(task).await;
        }
    }

    /// Start downloading a task
    async fn start_download(&self, mut task: DownloadTask) {
        let task_id = task.id.clone();
        let url = task.format.url.clone();
        let output_path = task.output_path.clone();

        // Create channels for progress updates and cancellation
        let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        // Update task status
        task.status = TaskStatus::Downloading;
        self.update_task_in_queue(task.clone()).await;

        // Clone engine for the task
        let engine = Arc::clone(&self.engine);
        let active_downloads = Arc::clone(&self.active_downloads);
        let queue = Arc::clone(&self.queue);

        // Start download task
        let join_handle = tokio::spawn(async move {
            let mut cancelled = false;

            // Create a future that completes when either the download finishes or is cancelled
            let download_task = engine.download(&url, &output_path, progress_tx.clone());
            let cancel_task = cancel_rx.recv();

            tokio::select! {
                result = download_task => {
                    match result {
                        Ok(()) => {
                            // Update task status to completed
                            task.status = TaskStatus::Completed;
                            info!("Task {} completed successfully", task_id);
                        }
                        Err(e) => {
                            // Update task status to failed
                            task.status = TaskStatus::Failed(e.to_string());
                            error!("Task {} failed: {}", task_id, e);
                        }
                    }
                }
                _ = cancel_task => {
                    // Task was cancelled
                    cancelled = true;
                    task.status = TaskStatus::Cancelled;
                    info!("Task {} was cancelled", task_id);
                }
            }

            // Update task in queue
            {
                let mut q = queue.lock().await;
                for t in q.iter_mut() {
                    if t.id == task_id {
                        t.status = task.status.clone();
                        break;
                    }
                }
            }

            // Remove from active downloads
            {
                let mut active = active_downloads.lock().await;
                active.remove(&task_id);
            }

            // If not cancelled, continue processing the queue
            if !cancelled {
                // This is a bit of a hack to continue processing the queue
                // In a real implementation, you might want a more elegant solution
                drop(active_downloads);
                drop(queue);
            }
        });

        // Add to active downloads
        {
            let mut active = self.active_downloads.lock().await;
            active.insert(
                task_id.clone(),
                DownloadHandle {
                    task_id: task_id.clone(),
                    join_handle,
                    cancel_tx,
                },
            );
        }

        // Handle progress updates
        let queue_clone = Arc::clone(&self.queue);
        let task_id_clone = task_id.clone();
        tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                // Update task progress
                {
                    let mut queue = queue_clone.lock().await;
                    for task in queue.iter_mut() {
                        if task.id == task_id_clone {
                            task.progress = Some(progress);
                            break;
                        }
                    }
                }
            }
        });

        info!("Started download for task {}", task_id);
    }

    /// Update task status in queue
    async fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<()> {
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
        let mut queue = self.queue.lock().await;
        for t in queue.iter_mut() {
            if t.id == task.id {
                *t = task;
                break;
            }
        }
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Queued
    }
}
            }
        }
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
