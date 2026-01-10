//! Data structures for video information

use serde::{Deserialize, Serialize};

/// Quality presets for quick selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityPreset {
    Best,
    FullHD, // 1080p
    HD,     // 720p
    SD,     // 480p
    AudioOnly,
}

impl QualityPreset {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Best => "Best",
            Self::FullHD => "1080p",
            Self::HD => "720p",
            Self::SD => "480p",
            Self::AudioOnly => "Audio",
        }
    }

    pub fn max_height(&self) -> Option<u32> {
        match self {
            Self::Best => None,
            Self::FullHD => Some(1080),
            Self::HD => Some(720),
            Self::SD => Some(480),
            Self::AudioOnly => None,
        }
    }

    pub fn all() -> &'static [QualityPreset] {
        &[
            Self::Best,
            Self::FullHD,
            Self::HD,
            Self::SD,
            Self::AudioOnly,
        ]
    }
}

/// Video information structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    #[serde(alias = "webpage_url")]
    pub url: String,
    #[serde(default)]
    pub direct_url: String, // Actual download URL (filled later)
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub filesize: Option<u64>,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub upload_date: Option<String>,
    #[serde(default)]
    pub formats: Vec<VideoFormat>,
    pub description: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub extractor: Option<String>,
    /// Recommended format ID (yt-dlp's "best")
    #[serde(alias = "format_id")]
    pub best_format_id: Option<String>,
}

impl VideoInfo {
    /// Get formats filtered by type
    pub fn video_formats(&self) -> Vec<&VideoFormat> {
        self.formats.iter().filter(|f| !f.audio_only).collect()
    }

    pub fn audio_formats(&self) -> Vec<&VideoFormat> {
        self.formats.iter().filter(|f| f.audio_only).collect()
    }

    pub fn combined_formats(&self) -> Vec<&VideoFormat> {
        self.formats.iter().filter(|f| f.is_combined()).collect()
    }

    /// Get format by ID
    pub fn get_format(&self, format_id: &str) -> Option<&VideoFormat> {
        self.formats.iter().find(|f| f.format_id == format_id)
    }

    /// Get best format matching criteria
    pub fn best_format_for_quality(&self, max_height: u32) -> Option<&VideoFormat> {
        self.combined_formats()
            .into_iter()
            .filter(|f| f.height.map(|h| h <= max_height).unwrap_or(false))
            .max_by_key(|f| f.height)
    }
}

/// Represents a single available format for a video
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoFormat {
    /// Format ID used by yt-dlp (e.g., "137", "140", "best")
    pub format_id: String,

    /// Human-readable format note (e.g., "1080p", "720p")
    pub format_note: Option<String>,

    /// File extension (e.g., "mp4", "webm", "m4a")
    pub ext: String,

    /// Video resolution (e.g., "1920x1080")
    pub resolution: Option<String>,

    /// Video height in pixels (e.g., 1080, 720)
    pub height: Option<u32>,

    /// Video width in pixels
    pub width: Option<u32>,

    /// File size in bytes (if known)
    pub filesize: Option<u64>,

    /// Approximate file size in bytes
    pub filesize_approx: Option<u64>,

    /// Video codec (e.g., "h264", "vp9")
    pub vcodec: Option<String>,

    /// Audio codec (e.g., "aac", "opus")
    pub acodec: Option<String>,

    /// Total bitrate in kbps
    pub tbr: Option<f64>,

    /// Video bitrate in kbps
    pub vbr: Option<f64>,

    /// Audio bitrate in kbps
    pub abr: Option<f64>,

    /// Frames per second
    pub fps: Option<f64>,

    /// Whether this is video-only (no audio)
    #[serde(default)]
    pub video_only: bool,

    /// Whether this is audio-only (no video)
    #[serde(default)]
    pub audio_only: bool,

    /// Direct URL for this format
    #[serde(default)]
    pub url: String,
}

impl VideoFormat {
    /// Returns estimated file size in bytes
    pub fn estimated_size(&self) -> Option<u64> {
        self.filesize.or(self.filesize_approx)
    }

    /// Returns human-readable size (e.g., "150 MB")
    pub fn size_string(&self) -> String {
        match self.estimated_size() {
            Some(bytes) => format_bytes(bytes),
            None => "Unknown".to_string(),
        }
    }

    /// Returns display label (e.g., "1080p MP4 - 150 MB")
    pub fn display_label(&self) -> String {
        let quality = self
            .format_note
            .clone()
            .or_else(|| self.height.map(|h| format!("{}p", h)))
            .unwrap_or_else(|| "Unknown".to_string());

        let size = self.size_string();
        let codec = self.codec_string();

        if self.audio_only {
            format!(
                "🎵 Audio {} - {} - {}",
                self.ext.to_uppercase(),
                size,
                codec
            )
        } else if self.video_only {
            format!(
                "🎬 {} {} (video only) - {} - {}",
                quality,
                self.ext.to_uppercase(),
                size,
                codec
            )
        } else {
            format!(
                "📹 {} {} - {} - {}",
                quality,
                self.ext.to_uppercase(),
                size,
                codec
            )
        }
    }

    /// Returns codec info string
    pub fn codec_string(&self) -> String {
        match (&self.vcodec, &self.acodec) {
            (Some(v), Some(a)) => format!("{}/{}", v, a),
            (Some(v), None) => v.clone(),
            (None, Some(a)) => a.clone(),
            (None, None) => "unknown".to_string(),
        }
    }

    /// Check if this is a "combined" format (has both video and audio)
    pub fn is_combined(&self) -> bool {
        !self.video_only && !self.audio_only
    }
}

/// Helper function to format bytes
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub type Format = VideoFormat; // Backward compatibility alias

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_label() {
        let fmt = VideoFormat {
            format_id: "137".to_string(),
            ext: "mp4".to_string(),
            resolution: Some("1920x1080".to_string()),
            height: Some(1080),
            filesize: Some(150_000_000), // ~143 MB
            vcodec: Some("h264".to_string()),
            acodec: Some("aac".to_string()),
            ..Default::default()
        };

        let label = fmt.display_label();
        // Exact format might depend on implementation details, checking key parts
        assert!(label.contains("1080p"));
        assert!(label.contains("MP4"));
        assert!(label.contains("MB"));
        assert!(label.contains("h264/aac"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_quality_presets() {
        assert_eq!(QualityPreset::FullHD.max_height(), Some(1080));
        assert_eq!(QualityPreset::Best.max_height(), None);
    }
}
