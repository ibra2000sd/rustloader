//! yt-dlp wrapper for video extraction
//!
//! This module handles video information extraction using yt-dlp.
//! It supports both bundled yt-dlp (in macOS .app bundles) and system-installed yt-dlp.

use crate::extractor::models::{Format, VideoInfo};
use crate::extractor::traits::Extractor;
use crate::utils::error::RustloaderError;
use anyhow::Result;
use async_trait::async_trait;
use serde_json;
use std::path::PathBuf;
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, error, info};

/// Main video extractor using yt-dlp
pub struct YtDlpExtractor {
    ytdlp_path: PathBuf,
}

impl YtDlpExtractor {
    /// Initialize extractor and verify yt-dlp availability
    ///
    /// Search order:
    /// 1. Bundled yt-dlp (inside .app bundle for macOS)
    /// 2. System PATH
    /// 3. Common installation paths (Homebrew, etc.)
    pub fn new() -> Result<Self> {
        let ytdlp_path = match find_ytdlp() {
            Some(path) => {
                info!("Found yt-dlp at: {}", path.display());
                path
            }
            None => {
                error!("yt-dlp not found anywhere!");
                return Err(RustloaderError::YtDlpNotFound.into());
            }
        };

        Ok(Self { ytdlp_path })
    }

    /// Extract video information without downloading
    /// Uses: yt-dlp --dump-json --no-download
    pub async fn extract_info_impl(&self, url: &str) -> Result<VideoInfo> {
        debug!("Extracting video info for URL: {}", url);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-warnings")
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
    pub async fn extract_playlist_impl(&self, url: &str) -> Result<Vec<VideoInfo>> {
        debug!("Extracting playlist info for URL: {}", url);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--no-warnings")
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
    pub async fn get_formats_impl(&self, url: &str) -> Result<Vec<Format>> {
        let video_info = self.extract_info_impl(url).await?;
        Ok(video_info.formats)
    }

    /// Search videos (YouTube)
    /// Uses: yt-dlp "ytsearch{count}:{query}"
    pub async fn search(&self, query: &str, count: usize) -> Result<Vec<VideoInfo>> {
        debug!("Searching for: {} (count: {})", query, count);

        let search_query = format!("ytsearch{}:{}", count, query);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("--dump-json")
            .arg("--no-warnings")
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

        Ok(vec![video_info])
    }

    /// Get direct download URL for a video with specific format
    pub async fn get_direct_url_impl(&self, url: &str, format_id: &str) -> Result<String> {
        debug!("Getting direct URL for format {} from {}", format_id, url);

        let output = AsyncCommand::new(&self.ytdlp_path)
            .arg("-f")
            .arg(format_id)
            .arg("-g")
            .arg("--no-warnings")
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

    pub fn ytdlp_path(&self) -> &PathBuf {
        &self.ytdlp_path
    }
}

#[async_trait]
impl Extractor for YtDlpExtractor {
    fn id(&self) -> &'static str {
        "yt-dlp"
    }

    fn supports(&self, _url: &str) -> bool {
        // yt-dlp supports almost everything, so we return true as a fallback
        // The HybridExtractor will prioritize other extractors first
        true
    }

    async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
        self.extract_info_impl(url).await
    }

    async fn extract_playlist(&self, url: &str) -> Result<Vec<VideoInfo>> {
        self.extract_playlist_impl(url).await
    }

    async fn get_formats(&self, url: &str) -> Result<Vec<Format>> {
        self.get_formats_impl(url).await
    }

    async fn get_direct_url(&self, url: &str, format_id: &str) -> Result<String> {
        self.get_direct_url_impl(url, format_id).await
    }
}

impl Default for YtDlpExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to initialize YtDlpExtractor")
    }
}

// ============================================================
// yt-dlp Detection Functions
// ============================================================

// ============================================================
// yt-dlp Detection Functions
// ============================================================

/// Find yt-dlp binary using platform abstraction
pub fn find_ytdlp() -> Option<PathBuf> {
    if let Some(path) = crate::utils::platform::ytdlp_path() {
        return Some(path);
    }

    // Fallback: Check common paths (legacy/development support)
    // Only keeping platform-agnostic or strictly necessary fallbacks if "which" fails
    // but typically platform::ytdlp_path covering 'which' is sufficient.
    // We can keep a minimal fallback if desired, or just return None.

    // Attempt manual common paths check if system PATH lookup failed via 'which'
    find_in_common_paths()
}

/// Find yt-dlp in common installation paths as a last resort
fn find_in_common_paths() -> Option<PathBuf> {
    let common_paths = if cfg!(target_os = "macos") {
        vec![
            "/opt/homebrew/bin/yt-dlp",
            "/usr/local/bin/yt-dlp",
            "/usr/bin/yt-dlp",
            "~/.local/bin/yt-dlp",
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            "/usr/bin/yt-dlp",
            "/usr/local/bin/yt-dlp",
            "~/.local/bin/yt-dlp",
            "/snap/bin/yt-dlp",
        ]
    } else {
        vec![]
    };

    for path_str in common_paths {
        // Expand ~ to home directory
        let expanded = if path_str.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                home.join(&path_str[2..])
            } else {
                PathBuf::from(path_str)
            }
        } else {
            PathBuf::from(path_str)
        };

        if expanded.exists() {
            return Some(expanded);
        }
    }

    None
}

/// Create a Command for yt-dlp with the correct path
pub fn ytdlp_command() -> Option<Command> {
    let ytdlp_path = find_ytdlp()?;
    Some(Command::new(ytdlp_path))
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_ytdlp() {
        let result = find_ytdlp();
        println!("yt-dlp found at: {:?}", result);
        // Don't assert - yt-dlp might not be installed in CI
    }

    #[test]
    fn test_find_in_common_paths() {
        let result = find_in_common_paths();
        println!("Common path yt-dlp: {:?}", result);
    }

    #[test]
    fn test_ytdlp_command() {
        if let Some(mut cmd) = ytdlp_command() {
            let output = cmd.arg("--version").output();
            if let Ok(out) = output {
                println!("yt-dlp version: {}", String::from_utf8_lossy(&out.stdout));
            }
        }
    }
}
