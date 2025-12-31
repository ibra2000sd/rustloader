//! yt-dlp wrapper for video extraction
#![allow(dead_code, unused_variables, unused_imports)]

use crate::extractor::models::{Format, VideoInfo};
use crate::utils::error::RustloaderError;
use anyhow::Result;
use serde_json;
use std::path::PathBuf;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, error, info};
use which::which;

/// Main video extractor using yt-dlp
pub struct VideoExtractor {
    ytdlp_path: PathBuf,
}

impl VideoExtractor {
    /// Initialize extractor and verify yt-dlp availability
    pub fn new() -> Result<Self> {
        // Try to find yt-dlp in the system PATH
        let ytdlp_path = match which("yt-dlp") {
            Ok(path) => path,
            Err(_) => {
                error!("yt-dlp not found in PATH");
                return Err(RustloaderError::YtDlpNotFound.into());
            }
        };

        info!("Found yt-dlp at: {}", ytdlp_path.display());

        Ok(Self { ytdlp_path })
    }

    /// Extract video information without downloading
    /// Uses: yt-dlp --dump-json --no-download
    pub async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
        debug!("Extracting video info for URL: {}", url);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("--dump-json")
            .arg("--no-download")
            .arg(url)
            .output()
            .await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            error!("yt-dlp extraction failed: {}", error_msg);
            return Err(RustloaderError::ExtractionError(error_msg.to_string()).into());
        }

        let json_str = String::from_utf8(output.stdout)?;
        let video_info: VideoInfo = serde_json::from_str(&json_str)?;

        Ok(video_info)
    }

    /// Extract playlist information
    /// Uses: yt-dlp --flat-playlist --dump-json
    pub async fn extract_playlist(&self, url: &str) -> Result<Vec<VideoInfo>> {
        debug!("Extracting playlist info for URL: {}", url);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg(url)
            .output()
            .await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            error!("yt-dlp playlist extraction failed: {}", error_msg);
            return Err(RustloaderError::ExtractionError(error_msg.to_string()).into());
        }

        let json_str = String::from_utf8(output.stdout)?;
        let lines = json_str.lines();
        let mut videos = Vec::new();

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<VideoInfo>(line) {
                Ok(video) => videos.push(video),
                Err(e) => {
                    error!("Failed to parse video info: {}", e);
                    // Continue with other videos
                }
            }
        }

        Ok(videos)
    }

    /// Get available formats for a video
    pub async fn get_formats(&self, url: &str) -> Result<Vec<Format>> {
        let video_info = self.extract_info(url).await?;
        Ok(video_info.formats)
    }

    /// Search videos (YouTube)
    /// Uses: yt-dlp "ytsearch{count}:{query}"
    pub async fn search(&self, query: &str, count: usize) -> Result<Vec<VideoInfo>> {
        debug!("Searching for: {} (count: {})", query, count);

        let search_query = format!("ytsearch{}:{}", count, query);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("--dump-json")
            .arg(search_query)
            .output()
            .await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            error!("yt-dlp search failed: {}", error_msg);
            return Err(RustloaderError::ExtractionError(error_msg.to_string()).into());
        }

        let json_str = String::from_utf8(output.stdout)?;
        let video_info: VideoInfo = serde_json::from_str(&json_str)?;

        // For search results, the response contains a single video with an "entries" field
        if !video_info.formats.is_empty() {
            // TODO: handle search result entries in detail when backend supports playlists
        }

        // For now, return a simple implementation
        Ok(vec![video_info])
    }

    /// Get direct download URL for a video with specific format
    pub async fn get_direct_url(&self, url: &str, format_id: &str) -> Result<String> {
        debug!("Getting direct URL for format {} from {}", format_id, url);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("-f")
            .arg(format_id)
            .arg("-g")
            .arg(url)
            .output()
            .await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            error!("Failed to get direct URL: {}", error_msg);
            return Err(RustloaderError::ExtractionError(error_msg.to_string()).into());
        }

        let url_str = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(url_str)
    }
}

/// Parse a raw yt-dlp JSON response into a `VideoInfo` structure.
/// This helper is pure and testable without requiring the yt-dlp binary.
pub fn parse_video_info(json: &str) -> Result<VideoInfo> {
    let info: VideoInfo = serde_json::from_str(json)?;
    Ok(info)
}

/// Select the format closest to (but not exceeding) a target height.
/// Falls back to the smallest available height when no candidate is below the target.
pub fn select_best_format(formats: &[Format], target_height: u32) -> Option<Format> {
    let mut with_height: Vec<&Format> = formats.iter().filter(|f| f.height.is_some()).collect();

    if with_height.is_empty() {
        return None;
    }

    with_height.sort_by_key(|f| f.height.unwrap_or(0));

    let best_below = with_height
        .iter()
        .rev()
        .find(|f| f.height.unwrap_or(0) <= target_height)
        .copied();

    best_below
        .cloned()
        .or_else(|| with_height.first().copied().cloned())
}

/// Build the command-line arguments used to extract metadata via yt-dlp.
/// Returned as a Vec<String> for easy inspection and testing.
pub fn build_extract_command(url: &str) -> Vec<String> {
    vec![
        "yt-dlp".to_string(),
        "--dump-json".to_string(),
        "--no-download".to_string(),
        url.to_string(),
    ]
}

impl Default for VideoExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to initialize VideoExtractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_video_info_handles_valid_json() {
        let json = r#"{
            "id": "abc123",
            "title": "Test Video",
            "url": "https://example.com/watch?v=abc123",
            "direct_url": "",
            "duration": 120,
            "filesize": null,
            "thumbnail": null,
            "uploader": "Uploader",
            "upload_date": "20240101",
            "formats": [
                {"format_id": "18", "ext": "mp4", "resolution": "360p", "filesize": null, "url": "https://cdn/18", "quality": null, "fps": null, "vcodec": null, "acodec": null, "format_note": null, "width": null, "height": 360, "tbr": null, "vbr": null, "abr": null},
                {"format_id": "22", "ext": "mp4", "resolution": "720p", "filesize": null, "url": "https://cdn/22", "quality": null, "fps": null, "vcodec": null, "acodec": null, "format_note": null, "width": null, "height": 720, "tbr": null, "vbr": null, "abr": null}
            ],
            "description": null,
            "view_count": null,
            "like_count": null,
            "extractor": "youtube"
        }"#;

        let info = parse_video_info(json).expect("should parse json");
        assert_eq!(info.id, "abc123");
        assert_eq!(info.title, "Test Video");
        assert_eq!(info.duration, Some(120));
        assert_eq!(info.formats.len(), 2);
    }

    #[test]
    fn parse_video_info_rejects_invalid_json() {
        let result = parse_video_info("not-json");
        assert!(result.is_err());
    }

    #[test]
    fn select_best_format_prefers_target_or_lower() {
        let formats = vec![
            Format {
                format_id: "18".into(),
                ext: "mp4".into(),
                resolution: Some("360p".into()),
                filesize: None,
                url: "https://cdn/18".into(),
                quality: None,
                fps: None,
                vcodec: None,
                acodec: None,
                format_note: None,
                width: None,
                height: Some(360),
                tbr: None,
                vbr: None,
                abr: None,
            },
            Format {
                format_id: "22".into(),
                ext: "mp4".into(),
                resolution: Some("720p".into()),
                filesize: None,
                url: "https://cdn/22".into(),
                quality: None,
                fps: None,
                vcodec: None,
                acodec: None,
                format_note: None,
                width: None,
                height: Some(720),
                tbr: None,
                vbr: None,
                abr: None,
            },
            Format {
                format_id: "137".into(),
                ext: "mp4".into(),
                resolution: Some("1080p".into()),
                filesize: None,
                url: "https://cdn/137".into(),
                quality: None,
                fps: None,
                vcodec: None,
                acodec: None,
                format_note: None,
                width: None,
                height: Some(1080),
                tbr: None,
                vbr: None,
                abr: None,
            },
        ];

        let best = select_best_format(&formats, 720).expect("format expected");
        assert_eq!(best.format_id, "22");

        let best_low = select_best_format(&formats, 480).expect("format expected");
        assert_eq!(best_low.format_id, "18");
    }

    #[test]
    fn build_extract_command_contains_core_flags() {
        let url = "https://example.com/watch?v=test";
        let cmd = build_extract_command(url);
        assert!(cmd.contains(&"yt-dlp".to_string()));
        assert!(cmd.contains(&"--dump-json".to_string()));
        assert!(cmd.contains(&url.to_string()));
    }
}
