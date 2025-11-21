//! Application configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Download location
    pub download_location: PathBuf,

    /// Number of segments per download
    pub segments: usize,

    /// Maximum concurrent downloads
    pub max_concurrent: usize,

    /// Preferred video quality
    pub quality: VideoQuality,

    /// Chunk size for streaming (bytes)
    pub chunk_size: usize,

    /// Retry attempts per segment
    pub retry_attempts: usize,

    /// Enable resume capability
    pub enable_resume: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            download_location: dirs::download_dir().unwrap_or_else(|| PathBuf::from("./downloads")),
            segments: 16,
            max_concurrent: 5,
            quality: VideoQuality::Best,
            chunk_size: 8192, // 8KB
            retry_attempts: 3,
            enable_resume: true,
        }
    }
}

/// Video quality options
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VideoQuality {
    Best,
    Worst,
    Specific(String), // Format ID
}

impl VideoQuality {
    /// Get string representation for display
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoQuality::Best => "Best Available",
            VideoQuality::Worst => "Worst Available",
            VideoQuality::Specific(_) => "Custom",
        }
    }
}
