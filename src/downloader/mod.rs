//! Download engine module

pub mod engine;
pub mod segment;
pub mod merger;
pub mod progress;

// Re-export for convenience
pub use engine::{DownloadEngine, DownloadConfig};
pub use progress::{DownloadProgress, DownloadStatus};
