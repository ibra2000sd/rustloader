//! Rustloader library

pub mod app;
pub mod database;
pub mod downloader;
pub mod extractor;
pub mod gui;
pub mod queue;
pub mod utils;
pub mod backend;

// Re-export main types for easier use
pub use downloader::{DownloadConfig, DownloadEngine, DownloadProgress, DownloadStatus};
pub use extractor::{Format, HybridExtractor, VideoInfo, YtDlpExtractor};
pub use gui::{Message, RustloaderApp, View};
pub use queue::{DownloadTask, QueueManager, TaskStatus};
pub use utils::{AppSettings, RustloaderError};
