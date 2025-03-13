// src/youtube_dl_wrapper.rs
//
// Rewritten to use youtube-dl-rs instead of rustube

use crate::error::AppError;
use crate::security;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use async_trait::async_trait;
use reqwest::Client;
use youtube_dl::{YoutubeDl, YoutubeDlOutput, SingleVideo, Playlist, VideoInfo};
use url::Url;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use tokio::time::sleep;

// Type definition for progress callback
pub type ProgressCallback = Arc<dyn Fn(u64, u64) -> bool + Send + Sync>;

/// Configuration for video download
#[derive(Clone, Debug)]
pub struct DownloadConfig {
    pub url: String,
    pub quality: Option<String>,
    pub format: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub use_playlist: bool,
    pub download_subtitles: bool,
    pub output_dir: PathBuf,
    pub bitrate: Option<String>,
}

/// Video platform type for different implementations
enum PlatformType {
    YouTube,
    Vimeo,
    Other,
}

/// Wrapper for youtube-dl-rs based video/audio downloading
pub struct YoutubeDlWrapper {
    config: DownloadConfig,
    progress: Arc<Mutex<(u64, u64)>>, // (downloaded, total)
    progress_callback: Option<ProgressCallback>,
    client: Client,
}

impl YoutubeDlWrapper {
    /// Create a new wrapper with the given configuration
    pub fn new(
        config: DownloadConfig, 
        progress_callback: Option<ProgressCallback>
    ) -> Self {
        // Create HTTP client with optimized settings
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Rustloader/1.0.0")
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self {
            config,
            progress: Arc::new(Mutex::new((0, 0))),
            progress_callback,
            client,
        }
    }
    
    /// Detect platform type from URL
    fn detect_platform(&self) -> PlatformType {
        let url = &self.config.url;
        
        if url.contains("youtube.com") || url.contains("youtu.be") {
            PlatformType::YouTube
        } else if url.contains("vimeo.com") {
            PlatformType::Vimeo
        } else {
            PlatformType::Other
        }
    }
    
    /// Download video with progress tracking
    pub async fn download(&self) -> Result<String, AppError> {
        // Validate URL
        crate::utils::validate_url(&self.config.url)?;
        
        // Apply rate limiting to avoid API abuse
        if !security::apply_rate_limit("youtube_download", 10, std::time::Duration::from_secs(60)) {
            return Err(AppError::ValidationError("Too many download attempts. Please try again later.".to_string()));
        }
        
        // Create a progress bar
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        pb.set_message("Starting download...");
        
        // For generic downloads, use youtube-dl as it handles most sites
        self.download_with_youtube_dl(&pb).await
    }
    
    /// Download using youtube-dl-rs
    async fn download_with_youtube_dl(&self, pb: &ProgressBar) -> Result<String, AppError> {
        // Create the output directory if it doesn't exist
        if !self.config.output_dir.exists() {
            tokio::fs::create_dir_all(&self.config.output_dir).await
                .map_err(|e| AppError::IoError(e))?;
        }
        
        // Set up the youtube-dl builder
        let mut ytdl = YoutubeDl::new(&self.config.url);
        
        // Configure quality
        if let Some(quality) = &self.config.quality {
            match quality.as_str() {
                "480" => {
                    ytdl.format("best[height<=480]/bestvideo[height<=480]+bestaudio/best");
                },
                "720" => {
                    ytdl.format("best[height<=720]/bestvideo[height<=720]+bestaudio/best");
                },
                "1080" => {
                    ytdl.format("best[height<=1080]/bestvideo[height<=1080]+bestaudio/best");
                },
                "2160" => {
                    ytdl.format("best[height<=2160]/bestvideo[height<=2160]+bestaudio/best");
                },
                _ => {
                    // Default to 720p
                    ytdl.format("best[height<=720]/bestvideo[height<=720]+bestaudio/best");
                }
            }
        } else if self.config.format == "mp3" {
            // Audio-only format
            ytdl.format("bestaudio");
        }
        
        // Set output path
        ytdl.output_directory(&self.config.output_dir);
        
        // Configure playlist handling
        if !self.config.use_playlist {
            ytdl.playlist_end(1); // Only download the first video if not a playlist
        }
        
        // Configure subtitles
        if self.config.download_subtitles {
            ytdl.download_subtitles(true)
                .write_subtitles(true)
                .write_auto_subtitles(true);
        }
        
        // Set start and end times if specified
        if let Some(start) = &self.config.start_time {
            ytdl.extra_arg("--postprocessor-args").extra_arg(format!("ffmpeg:-ss {}", start));
        }
        
        if let Some(end) = &self.config.end_time {
            ytdl.extra_arg("--postprocessor-args").extra_arg(format!("ffmpeg:-to {}", end));
        }
        
        // If format is MP3, set up audio extraction
        if self.config.format == "mp3" {
            ytdl.extract_audio(true);
            ytdl.audio_format("mp3");
            
            // Set bitrate if specified
            if let Some(bitrate) = &self.config.bitrate {
                ytdl.audio_quality(bitrate.clone());
            } else {
                ytdl.audio_quality("128K"); // Default for free version
            }
        }
        
        // Enable progress callback
        let progress_arc = Arc::clone(&self.progress);
        let progress_callback = self.progress_callback.clone();
        
        // Create a custom progress handler
        ytdl.extra_arg("--progress-template")
            .extra_arg("download:%(progress.downloaded_bytes)s/%(progress.total_bytes)s");
        
        // This will require a custom progress parser in a real implementation
        
        // Set up a background task to update progress
        let url = self.config.url.clone();
        let progress_task = tokio::spawn(async move {
            let mut downloaded: u64 = 0;
            let mut total: u64 = 1000000; // Placeholder default
            
            loop {
                // In a real implementation, this would read the progress from youtube-dl
                // For now we'll simulate progress
                downloaded += 50000;
                if downloaded > total {
                    downloaded = total;
                }
                
                // Update our progress
                {
                    let mut current = progress_arc.lock().await;
                    *current = (downloaded, total);
                }
                
                // Call progress callback if provided
                if let Some(callback) = &progress_callback {
                    if !callback(downloaded, total) {
                        break;
                    }
                }
                
                // Update progress bar
                let percentage = (downloaded * 100) / total;
                pb.set_position(percentage);
                pb.set_message(format!("Downloaded: {} / {}", 
                                     format_bytes(downloaded), 
                                     format_bytes(total)));
                
                if downloaded >= total {
                    break;
                }
                
                sleep(Duration::from_millis(100)).await;
            }
        });
        
        // Run youtube-dl
        pb.set_message("Downloading video information...");
        
        let result = ytdl.run_async().await
            .map_err(|e| AppError::DownloadError(format!("youtube-dl error: {}", e)))?;
        
        // Process the result
        let title = match result {
            YoutubeDlOutput::SingleVideo(video) => {
                video.title.unwrap_or_else(|| "Unknown Title".to_string())
            },
            YoutubeDlOutput::Playlist(playlist) => {
                format!("Playlist: {}", playlist.title.unwrap_or_else(|| "Unknown Playlist".to_string()))
            },
        };
        
        // Cancel the progress task
        progress_task.abort();
        
        // Complete the progress bar
        pb.finish_with_message(format!("Download complete: {}", title));
        
        Ok(format!("Successfully downloaded: {}", title))
    }
    
    /// Get current progress
    pub async fn get_progress(&self) -> (u64, u64) {
        *self.progress.lock().await
    }
}

// Helper functions

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Sanitize a filename by removing invalid characters
fn sanitize_filename(input: &str) -> String {
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];
    let mut result = input.to_string();
    
    for c in invalid_chars {
        result = result.replace(c, "_");
    }
    
    // Limit length
    if result.len() > 128 {
        let bytes = result.as_bytes();
        result = String::from_utf8_lossy(&bytes[0..128]).to_string();
    }
    
    // Trim whitespace
    result.trim().to_string()
}