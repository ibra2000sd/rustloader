// src/lib.rs
// Add the new modules

pub mod cli;
pub mod downloader;
pub mod dependency_validator;
pub mod error;
pub mod security;
pub mod utils;
pub mod license;
pub mod ytdlp_wrapper;  // Changed from youtube_dl_wrapper
pub mod ffmpeg_wrapper;
pub mod promo;         // New module for promotional messages
pub mod counter;       // New module for download counting

// Re-export VERSION from main.rs to make it accessible throughout the crate
pub const VERSION: &str = "1.0.0";

// You can optionally re-export commonly used types
pub use error::AppError;
pub use ytdlp_wrapper::{YtDlpWrapper, DownloadConfig};  // Changed
pub use downloader::DownloadProgress;