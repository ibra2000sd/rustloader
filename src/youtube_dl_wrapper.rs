// src/youtube_dl_wrapper.rs

use crate::error::AppError;
use crate::security;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use youtube_dl::{YoutubeDl, YoutubeDlOutput};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::thread;
use std::sync::mpsc;

// Type definition for progress callback
pub type ProgressCallback = Arc<dyn Fn(u64, u64) -> bool + Send + Sync>;

/// Configuration for YouTube download
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

/// Wrapper for youtube-dl-rs with progress tracking
pub struct YoutubeDlWrapper {
    config: DownloadConfig,
    progress: Arc<Mutex<(u64, u64)>>, // (downloaded, total)
    progress_callback: Option<ProgressCallback>,
}

impl YoutubeDlWrapper {
    /// Create a new wrapper with the given configuration
    pub fn new(
        config: DownloadConfig, 
        progress_callback: Option<ProgressCallback>
    ) -> Self {
        Self {
            config,
            progress: Arc::new(Mutex::new((0, 0))),
            progress_callback,
        }
    }
    
    /// Enable parallel downloads for faster processing
    fn enable_parallel_downloads(&mut self, ytdl: &mut YoutubeDl) {
        // Configure to use multiple threads for faster downloads
        ytdl.extra_arg("--concurrent-fragments").extra_arg("8");
        
        // Add additional optimizations for network and retries
        ytdl.extra_arg("--socket-timeout").extra_arg("30");
        ytdl.extra_arg("--retries").extra_arg("10");
    }
    
    /// Download video with progress tracking
    pub async fn download(&self) -> Result<String, AppError> {
        // Validate URL
        crate::utils::validate_url(&self.config.url)?;
        
        // Apply rate limiting to avoid API abuse
        if !security::apply_rate_limit("youtube_download", 10, std::time::Duration::from_secs(60)) {
            return Err(AppError::ValidationError("Too many download attempts. Please try again later.".to_string()));
        }
        
        // Build YouTube-DL options
        let mut ytdl = YoutubeDl::new(&self.config.url);
        
        // Enable parallel downloads for both free and pro versions
        let mut self_mut = self.clone();
        self_mut.enable_parallel_downloads(&mut ytdl);
        
        // Set extraction options
        ytdl.extra_arg("--extract-audio").extra_arg("--audio-format").extra_arg("mp3")
            .format(&self.get_format_string())
            .extra_arg("-o").extra_arg(self.get_output_template())
            .extra_arg("--no-warnings");
            
        // Add subtitles if requested
        if self.config.download_subtitles {
            ytdl.extra_arg("--write-sub");
        }
        
        // Handle playlist settings
        if !self.config.use_playlist {
            ytdl.extra_arg("--no-playlist");
        }
        
        // Add custom start/end time
        if let Some(start) = &self.config.start_time {
            ytdl.extra_arg("--external-downloader-args")
                .extra_arg(format!("ffmpeg:-ss {}", start));
        }
        
        if let Some(end) = &self.config.end_time {
            ytdl.extra_arg("--external-downloader-args")
                .extra_arg(format!("ffmpeg:-to {}", end));
        }
        
        // Configure output directory - convert PathBuf to string
        let output_dir_str = self.config.output_dir.to_string_lossy().to_string();
        ytdl.output_directory(output_dir_str);
        
        // Set up progress tracking using spawned thread and channel
        // since youtube_dl crate doesn't provide built-in progress tracking
        let progress_clone = Arc::clone(&self.progress);
        let callback_clone = self.progress_callback.clone();
        let url_clone = self.config.url.clone();
        
        // Create a channel for progress updates
        let (tx, rx) = mpsc::channel();
        
        // Spawn a thread to monitor download progress
        thread::spawn(move || {
            // Build a separate youtube-dl command to get progress info
            let mut cmd = Command::new("youtube-dl");
            cmd.arg("--newline")
               .arg("--progress-template")
               .arg("download:%(progress.downloaded_bytes)s/%(progress.total_bytes)s")
               .arg(&url_clone)
               .stdout(Stdio::piped())
               .stderr(Stdio::null());
            
            // Run the command
            match cmd.spawn() {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                if line.starts_with("download:") {
                                    if let Some(progress_str) = line.strip_prefix("download:") {
                                        let parts: Vec<&str> = progress_str.split('/').collect();
                                        if parts.len() == 2 {
                                            // Try to parse downloaded and total bytes
                                            if let (Ok(downloaded), Ok(total)) = (
                                                parts[0].trim().parse::<u64>(),
                                                parts[1].trim().parse::<u64>(),
                                            ) {
                                                // Send progress update through channel
                                                let _ = tx.send((downloaded, total));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Err(_) => { /* Command failed - we'll just continue without progress updates */ }
            }
        });
        
        // Spawn a task to process progress updates
        tokio::spawn(async move {
            while let Ok((downloaded, total)) = rx.try_recv() {
                if total > 0 {
                    // Update progress
                    let mut current = progress_clone.lock().await;
                    *current = (downloaded, total);
                    
                    // Call progress callback if provided
                    if let Some(callback) = &callback_clone {
                        if !callback(downloaded, total) {
                            // Callback returned false - ideally we'd cancel download
                            // but we can't directly interrupt the youtube-dl process
                        }
                    }
                }
                
                // Brief pause to prevent CPU spinning
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        // Run the download
        let output = ytdl.run()?;
        
        // Process the output
        match output {
            YoutubeDlOutput::SingleVideo(video) => {
                Ok(self.process_single_video_output(*video)) // Dereference the Box<SingleVideo>
            },
            YoutubeDlOutput::Playlist(playlist) => {
                Ok(self.process_playlist_output(*playlist)) // Dereference the Box<Playlist>
            }
        }
    }
    
    /// Get the format string based on configuration
    fn get_format_string(&self) -> String {
        if self.config.format == "mp3" {
            "bestaudio[ext=m4a]/bestaudio".to_string()
        } else {
            match self.config.quality.as_deref() {
                Some("480") => "best[height<=480]/bestvideo[height<=480]+bestaudio/best".to_string(),
                Some("720") => "best[height<=720]/bestvideo[height<=720]+bestaudio/best".to_string(),
                Some("1080") => "best[height<=1080]/bestvideo[height<=1080]+bestaudio/best".to_string(),
                Some("2160") => "best[height<=2160]/bestvideo[height<=2160]+bestaudio/best".to_string(),
                Some("4k") | Some("4K") => "best[height<=2160]/bestvideo[height<=2160]+bestaudio/best".to_string(),
                Some("8k") | Some("8K") => "best[height<=4320]/bestvideo[height<=4320]+bestaudio/best".to_string(),
                _ => "best".to_string()
            }
        }
    }
    
    /// Get output template string
    fn get_output_template(&self) -> String {
        let extension = if self.config.format == "mp3" { "mp3" } else { "mp4" };
        format!("%(title)s.{}", extension)
    }
    
    /// Process single video output
    fn process_single_video_output(&self, video: youtube_dl::SingleVideo) -> String {
        // Return the title or a default success message
        video.title.unwrap_or_else(|| "Download completed successfully".to_string())
    }
    
    /// Process playlist output
    fn process_playlist_output(&self, playlist: youtube_dl::Playlist) -> String {
        format!("Downloaded playlist: {} ({} videos)", 
                playlist.title.unwrap_or_else(|| "Unknown".to_string()),
                playlist.entries.map_or(0, |entries| entries.len()))
    }
    
    /// Get current progress
    pub async fn get_progress(&self) -> (u64, u64) {
        *self.progress.lock().await
    }
}

// Add Clone implementation for YoutubeDlWrapper
impl Clone for YoutubeDlWrapper {
    fn clone(&self) -> Self {
        Self {
            config: DownloadConfig {
                url: self.config.url.clone(),
                quality: self.config.quality.clone(),
                format: self.config.format.clone(),
                start_time: self.config.start_time.clone(),
                end_time: self.config.end_time.clone(),
                use_playlist: self.config.use_playlist,
                download_subtitles: self.config.download_subtitles,
                output_dir: self.config.output_dir.clone(),
                bitrate: self.config.bitrate.clone(),
            },
            progress: Arc::clone(&self.progress),
            progress_callback: self.progress_callback.clone(),
        }
    }
}

// Helper function to convert errors
impl From<youtube_dl::Error> for AppError {
    fn from(error: youtube_dl::Error) -> Self {
        match error {
            youtube_dl::Error::Json(e) => AppError::JsonError(e),
            youtube_dl::Error::Io(e) => AppError::IoError(e),
            youtube_dl::Error::ExitCode { code, stderr } => {
                if stderr.contains("HTTP Error 416") {
                    AppError::DownloadError("File already exists (HTTP 416)".to_string())
                } else if stderr.contains("dailyLimitExceeded") {
                    AppError::DailyLimitExceeded
                } else {
                    AppError::DownloadError(format!("YouTube-DL error (code {}): {}", code, stderr))
                }
            },
            _ => AppError::DownloadError("Unknown download error".to_string()),
        }
    }
}