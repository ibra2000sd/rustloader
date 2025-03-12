// src/lib.rs

pub mod cli;
pub mod downloader;
pub mod dependency_validator;
pub mod error;
pub mod security;
pub mod utils;
pub mod license;
pub mod youtube_dl_wrapper;
pub mod ffmpeg_wrapper;

// Re-export VERSION from main.rs to make it accessible throughout the crate
pub const VERSION: &str = "1.0.0";

// You can optionally re-export commonly used types
pub use error::AppError;
pub use youtube_dl_wrapper::{YoutubeDlWrapper, DownloadConfig};
pub use downloader::DownloadProgress;