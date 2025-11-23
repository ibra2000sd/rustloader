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
}

/// Video quality options
#[derive(Debug, Clone, Serialize, Deserialize)]
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
