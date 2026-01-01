//! yt-dlp wrapper for video extraction
//! 
//! This module handles video information extraction using yt-dlp.
//! It supports both bundled yt-dlp (in macOS .app bundles) and system-installed yt-dlp.

use crate::extractor::models::{VideoInfo, Format};
use crate::utils::error::RustloaderError;
use anyhow::Result;
use serde_json;
use std::path::PathBuf;
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, error, info, warn};

/// Main video extractor using yt-dlp
pub struct VideoExtractor {
    ytdlp_path: PathBuf,
}

impl VideoExtractor {
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
    pub async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
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
    pub async fn extract_playlist(&self, url: &str) -> Result<Vec<VideoInfo>> {
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
    pub async fn get_direct_url(&self, url: &str, format_id: &str) -> Result<String> {
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
    
    /// Get the path to yt-dlp being used
    pub fn ytdlp_path(&self) -> &PathBuf {
        &self.ytdlp_path
    }
}

impl Default for VideoExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to initialize VideoExtractor")
    }
}

// ============================================================
// yt-dlp Detection Functions
// ============================================================

/// Find yt-dlp binary with priority:
/// 1. Bundled (inside .app bundle)
/// 2. System PATH
/// 3. Common installation paths
pub fn find_ytdlp() -> Option<PathBuf> {
    // First: Check bundled yt-dlp (for macOS .app bundle)
    if let Some(bundled) = find_bundled_ytdlp() {
        info!("✓ Using bundled yt-dlp: {:?}", bundled);
        return Some(bundled);
    }
    
    // Second: Check PATH
    if let Some(system) = find_in_path() {
        info!("✓ Using system yt-dlp: {:?}", system);
        return Some(system);
    }
    
    // Third: Check common locations
    if let Some(common) = find_in_common_paths() {
        info!("✓ Using yt-dlp from common path: {:?}", common);
        return Some(common);
    }
    
    warn!("✗ yt-dlp not found anywhere!");
    None
}

/// Find bundled yt-dlp inside macOS .app bundle
fn find_bundled_ytdlp() -> Option<PathBuf> {
    // Get current executable path
    let exe_path = std::env::current_exe().ok()?;
    debug!("Current executable: {:?}", exe_path);
    
    // Get the directory containing the executable
    let exe_dir = exe_path.parent()?;
    
    // Check if we're in a MacOS directory (indicates .app bundle)
    // Structure: Rustloader.app/Contents/MacOS/rustloader_bin
    //                                  /Resources/bin/yt-dlp
    if exe_dir.ends_with("MacOS") {
        let contents_dir = exe_dir.parent()?;
        let ytdlp_path = contents_dir.join("Resources").join("bin").join("yt-dlp");
        
        debug!("Checking bundled path: {:?}", ytdlp_path);
        
        if ytdlp_path.exists() && ytdlp_path.is_file() {
            // Verify it's executable
            if is_executable(&ytdlp_path) {
                return Some(ytdlp_path);
            } else {
                warn!("Bundled yt-dlp exists but is not executable: {:?}", ytdlp_path);
            }
        }
    }
    
    // Also check if yt-dlp is next to the executable (for development)
    let dev_path = exe_dir.join("yt-dlp");
    if dev_path.exists() && is_executable(&dev_path) {
        return Some(dev_path);
    }
    
    None
}

/// Find yt-dlp in system PATH using `which`
fn find_in_path() -> Option<PathBuf> {
    // Try using the which crate first
    if let Ok(path) = which::which("yt-dlp") {
        if path.exists() {
            return Some(path);
        }
    }
    
    // Fallback: Use shell `which` command
    let output = Command::new("which")
        .arg("yt-dlp")
        .output()
        .ok()?;
    
    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(path_str.trim());
        if path.exists() {
            return Some(path);
        }
    }
    
    None
}

/// Find yt-dlp in common installation paths
fn find_in_common_paths() -> Option<PathBuf> {
    let common_paths = [
        // macOS Homebrew (Apple Silicon)
        "/opt/homebrew/bin/yt-dlp",
        // macOS Homebrew (Intel)
        "/usr/local/bin/yt-dlp",
        // System
        "/usr/bin/yt-dlp",
        // Python.org installation
        "/Library/Frameworks/Python.framework/Versions/Current/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.11/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.12/bin/yt-dlp",
        // User local
        "~/.local/bin/yt-dlp",
        // pip user install
        "/Users/*/Library/Python/*/bin/yt-dlp",
    ];
    
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
        
        if expanded.exists() && is_executable(&expanded) {
            return Some(expanded);
        }
    }
    
    None
}

/// Check if a file is executable
fn is_executable(path: &PathBuf) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        
        if let Ok(metadata) = std::fs::metadata(path) {
            let permissions = metadata.permissions();
            // Check if any executable bit is set
            return permissions.mode() & 0o111 != 0;
        }
    }
    
    #[cfg(not(unix))]
    {
        // On Windows, just check if file exists
        return path.exists();
    }
    
    false
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
    fn test_find_bundled_ytdlp() {
        let result = find_bundled_ytdlp();
        println!("Bundled yt-dlp: {:?}", result);
    }
    
    #[test]
    fn test_find_in_path() {
        let result = find_in_path();
        println!("System yt-dlp: {:?}", result);
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
    
    #[test]
    fn test_is_executable() {
        // Test with known executable
        let path = PathBuf::from("/bin/ls");
        if path.exists() {
            assert!(is_executable(&path));
        }
    }
}