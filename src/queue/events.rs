use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use tokio::fs::{OpenOptions, File};
use tokio::io::{AsyncWriteExt, BufWriter};
use crate::extractor::{VideoInfo, Format};
// use crate::queue::TaskStatus;
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Events that describe changes in the download queue state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueEvent {
    /// A new task was added to the queue
    TaskAdded {
        task_id: String,
        video_info: VideoInfo,
        format: Format,
        output_path: PathBuf,
        timestamp: DateTime<Utc>,
    },
    /// A task started downloading
    TaskStarted {
        task_id: String,
        timestamp: DateTime<Utc>,
    },
    /// A task was paused
    TaskPaused {
        task_id: String,
        timestamp: DateTime<Utc>,
    },
    /// A task was resumed
    TaskResumed {
        task_id: String,
        timestamp: DateTime<Utc>,
    },
    /// A task completed successfully
    TaskCompleted {
        task_id: String,
        output_path: PathBuf,
        timestamp: DateTime<Utc>,
    },
    /// A task failed
    TaskFailed {
        task_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// A task was cancelled/removed
    TaskRemoved {
        task_id: String,
        timestamp: DateTime<Utc>,
    },
}

/// Helper to manage the persistent event log
#[derive(Debug, Clone)]
pub struct EventLog {
    file_path: PathBuf,
    writer: Arc<Mutex<Option<BufWriter<File>>>>,
}

impl EventLog {
    pub async fn new(base_dir: &Path) -> Result<Self> {
        let file_path = base_dir.join("events.jsonl");
        
        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // We open the file in append mode. 
        // We defer opening the writer until the first write to avoid locking issues during rehydration? 
        // No, rehydration is read-only. We can just open it.
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
            .context("Failed to open event log")?;

        Ok(Self {
            file_path,
            writer: Arc::new(Mutex::new(Some(BufWriter::new(file)))),
        })
    }

    /// Append an event to the log
    pub async fn log(&self, event: QueueEvent) -> Result<()> {
        let mut guard = self.writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            let json = serde_json::to_string(&event)?;
            writer.write_all(json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?; // Flush immediately for safety
        }
        Ok(())
    }

    /// Read all events from the log for rehydration
    pub async fn read_events(&self) -> Result<Vec<QueueEvent>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&self.file_path).await?;
        let mut events = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() { continue; }
            match serde_json::from_str::<QueueEvent>(line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    tracing::warn!("Failed to parse event log line: {}. Error: {}", line, e);
                    // continue, don't break the whole app for one bad line
                }
            }
        }
        Ok(events)
    }
}
