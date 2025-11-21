//! Video extraction module using yt-dlp wrapper

pub mod ytdlp;
pub mod models;

// Re-export for convenience
pub use ytdlp::VideoExtractor;
pub use models::{VideoInfo, Format};
