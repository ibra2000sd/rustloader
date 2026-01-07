pub mod events;
pub mod manager;

pub use events::{EventLog, QueueEvent};
pub use manager::{DownloadTask, QueueManager, TaskStatus};
