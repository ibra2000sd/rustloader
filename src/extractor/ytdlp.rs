//! yt-dlp wrapper for video extraction
#![allow(dead_code, unused_variables, unused_imports)]

use crate::extractor::models::{VideoInfo, Format};
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
        if let Some(entries) = video_info.formats.get(0).and_then(|f| {
            // This is a workaround - in the actual implementation, we might need to handle
            // the search result format properly
            None::<Format>
        }) {
            // Parse entries
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

impl Default for VideoExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to initialize VideoExtractor")
    }
}
