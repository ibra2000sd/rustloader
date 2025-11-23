//! Data structures for video information

use serde::{Deserialize, Serialize};

/// Video information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    #[serde(alias = "webpage_url")]
    pub url: String,
    #[serde(default)]
    pub direct_url: String,  // Actual download URL (filled later)
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub filesize: Option<u64>,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub upload_date: Option<String>,
    #[serde(default)]
    pub formats: Vec<Format>,
    pub description: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub extractor: Option<String>,
}

/// Video format information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Format {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    #[serde(default)]
    pub filesize: Option<u64>,
    pub url: String,
    pub quality: Option<f32>,
    pub fps: Option<f32>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub format_note: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub tbr: Option<f32>,  // Total bitrate
    pub vbr: Option<f32>,  // Video bitrate
    pub abr: Option<f32>,  // Audio bitrate
}
