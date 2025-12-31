//! Video extraction module using yt-dlp wrapper

pub mod models;
pub mod ytdlp;

// Re-export for convenience
pub use models::{Format, VideoInfo};
pub use ytdlp::VideoExtractor;
