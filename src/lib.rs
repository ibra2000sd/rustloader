//! Rustloader library

pub mod app;
pub mod extractor;
pub mod downloader;
pub mod queue;
pub mod database;
pub mod gui;
pub mod utils;

// Re-export main types for easier use
pub use extractor::{VideoExtractor, VideoInfo, Format};
pub use downloader::{DownloadEngine, DownloadConfig, DownloadProgress, DownloadStatus};
pub use queue::{QueueManager, DownloadTask, TaskStatus};
pub use gui::{RustloaderApp, Message, View};
pub use utils::{RustloaderError, AppSettings};
