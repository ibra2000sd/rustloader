//! Queue management module

pub mod manager;

// Re-export for convenience
pub use manager::{QueueManager, DownloadTask, TaskStatus};
