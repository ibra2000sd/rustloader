// src/ytdlp_improved.rs

use crate::error::AppError;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

// Type definition for progress callback
pub type ProgressCallback = Arc<dyn Fn(u64, u64, &VideoInfo) -> bool + Send + Sync>;

/// Video metadata information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub uploader: String,
    pub duration: f64,
    pub upload_date: String,
    pub formats: Vec<FormatInfo>,
    pub thumbnail: String,
    pub description: String,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
}

/// Video format information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormatInfo {
    pub format_id: String,
    pub format_note: String,
    pub ext: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: Option<u64>,
}

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

/// Wrapper for yt-dlp command-line tool with enhanced features
pub struct YtDlpEnhanced {
    config: DownloadConfig,
    progress_callback: Option<ProgressCallback>,
    video_info: Arc<std::sync::Mutex<Option<VideoInfo>>>,
}

impl YtDlpEnhanced {
    /// Create a new wrapper with the given configuration
    pub fn new(
        config: DownloadConfig, 
        progress_callback: Option<ProgressCallback>
    ) -> Self {
        Self {
            config,
            progress_callback,
            video_info: Arc::new(std::sync::Mutex::new(None)),
        }
    }
    
    /// Extract video information before downloading
    pub async fn get_video_info(&self) -> Result<VideoInfo, AppError> {
        // If we already have info, return it
        if let Some(info) = self.video_info.lock().unwrap().clone() {
            return Ok(info);
        }
        
        // Build command to extract JSON info
        let mut cmd = Command::new("yt-dlp");
        
        cmd.arg("--dump-json")
           .arg("--no-playlist")
           .arg("--flat-playlist")
           .arg(&self.config.url);
        
        // Set up pipes
        cmd.stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        // Execute command
        let output = cmd.output().await
            .map_err(|e| AppError::DownloadError(format!("Failed to execute yt-dlp info command: {}", e)))?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::DownloadError(format!("yt-dlp info command failed: {}", error)));
        }
        
        // Parse JSON output
        let json = String::from_utf8_lossy(&output.stdout);
        let info: VideoInfo = match serde_json::from_str(&json) {
            Ok(info) => info,
            Err(e) => {
                // If we can't parse the full info, try extracting just the basic info
                let mut basic_info = VideoInfo::default();
                
                // Extract title using regex
                let title_regex = Regex::new(r#""title":\s*"([^"]*)""#).unwrap();
                if let Some(title_match) = title_regex.captures(&json) {
                    if let Some(title) = title_match.get(1) {
                        basic_info.title = title.as_str().to_string();
                    }
                }
                
                // Extract duration using regex
                let duration_regex = Regex::new(r#""duration":\s*([0-9.]+)"#).unwrap();
                if let Some(duration_match) = duration_regex.captures(&json) {
                    if let Some(duration) = duration_match.get(1) {
                        basic_info.duration = duration.as_str().parse().unwrap_or(0.0);
                    }
                }
                
                // Return partial info
                basic_info
            }
        };
        
        // Store info for later use
        *self.video_info.lock().unwrap() = Some(info.clone());
        
        Ok(info)
    }
    
    /// Download video with progress tracking
    pub async fn download(&self) -> Result<String, AppError> {
        // Try to get video info first
        let video_info = match self.get_video_info().await {
            Ok(info) => info,
            Err(_) => VideoInfo::default(),
        };
        
        // Create a progress bar
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        // Build the yt-dlp command with all necessary arguments
        let mut cmd = Command::new("yt-dlp");
        
        // Add basic arguments
        cmd.arg(self.config.url.clone())
            .arg("--progress")
            .arg("--newline")
            .arg("-o")
            .arg(self.config.output_dir.join("%(title)s.%(ext)s").to_string_lossy().to_string());
            
        // Configure quality
        if let Some(quality) = &self.config.quality {
            match quality.as_str() {
                "480" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=480]/bestvideo[height<=480]+bestaudio/best");
                },
                "720" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=720]/bestvideo[height<=720]+bestaudio/best");
                },
                "1080" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=1080]/bestvideo[height<=1080]+bestaudio/best");
                },
                "2160" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=2160]/bestvideo[height<=2160]+bestaudio/best");
                },
                _ => {
                    // Default to 720p
                    cmd.arg("-f");
                    cmd.arg("best[height<=720]/bestvideo[height<=720]+bestaudio/best");
                },
            }
        } else if self.config.format == "mp3" {
            // For audio, we'll use post-processing to convert to mp3
            cmd.arg("-f").arg("bestaudio")
               .arg("-x").arg("--audio-format").arg("mp3");
            
            // Add bitrate if specified
            if let Some(bitrate) = &self.config.bitrate {
                cmd.arg("--audio-quality").arg(bitrate);
            } else {
                cmd.arg("--audio-quality").arg("128K"); // Default for free version
            }
        }
        
        // Configure playlist handling
        if !self.config.use_playlist {
            cmd.arg("--no-playlist");
        }
        
        // Configure subtitles
        if self.config.download_subtitles {
            cmd.arg("--write-subs").arg("--write-auto-subs");
        }
        
        // Configure time segments
        let mut ffmpeg_args = Vec::new();
        
        if let Some(start) = &self.config.start_time {
            ffmpeg_args.push(format!("-ss {}", start));
        }
        
        if let Some(end) = &self.config.end_time {
            ffmpeg_args.push(format!("-to {}", end));
        }
        
        if !ffmpeg_args.is_empty() {
            cmd.arg("--postprocessor-args");
            cmd.arg(format!("ffmpeg:{}", ffmpeg_args.join(" ")));
        }
        
        // Set up stdin, stdout, and stderr for capturing output
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        println!("{} {}", "Running yt-dlp command:".blue(), format!("{:?}", cmd).yellow());
        
        // Execute the command
        let mut child = cmd.spawn()
            .map_err(|e| AppError::DownloadError(format!("Failed to execute yt-dlp: {}", e)))?;
        
        // Set up Atomics for progress tracking
        let downloaded = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(100));
        let percentage = Arc::new(AtomicU64::new(0));
        let title = Arc::new(std::sync::Mutex::new(video_info.title.clone()));
        
        // Clone for progress callback
        let progress_callback = self.progress_callback.clone();
        let video_info_clone = video_info.clone();
        
        // Set up a timer for progress updates
        let progress_task = {
            let downloaded = downloaded.clone();
            let total = total.clone();
            let percentage = percentage.clone();
            let pb = pb.clone();
            let video_info = video_info_clone.clone();
            
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    
                    let dl = downloaded.load(Ordering::Relaxed);
                    let tot = total.load(Ordering::Relaxed);
                    let pct = percentage.load(Ordering::Relaxed);
                    
                    pb.set_position(pct);
                    pb.set_message(format!("Downloaded: {} / {}", 
                                         format_bytes(dl), 
                                         format_bytes(tot)));
                    
                    // Call progress callback if provided
                    if let Some(callback) = &progress_callback {
                        if !callback(dl, tot, &video_info) {
                            break;
                        }
                    }
                    
                    if pct >= 100 {
                        break;
                    }
                }
            })
        };
        
        // Process stdout to capture progress information
        if let Some(stdout) = child.stdout.take() {
            let downloaded = downloaded.clone();
            let total = total.clone();
            let percentage = percentage.clone();
            let title_clone = title.clone();
            
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                
                let re_progress = Regex::new(r"(\d+\.\d+)% of ~?(\d+\.\d+)(\w+) at\s+(\d+\.\d+)(\w+)/s").unwrap();
                let re_title = Regex::new(r"\[download\] Destination: (.+?)(?:\.\w+)?$").unwrap();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Process progress update
                    if let Some(caps) = re_progress.captures(&line) {
                        if let (Some(percent_str), Some(size_str), Some(size_unit), Some(_speed_str), Some(_speed_unit)) = 
                            (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5)) {
                            
                            // Parse percentage
                            if let Ok(percent) = percent_str.as_str().parse::<f64>() {
                                percentage.store(percent as u64, Ordering::Relaxed);
                            }
                            
                            // Parse total size
                            if let Ok(size) = size_str.as_str().parse::<f64>() {
                                let size_bytes = match size_unit.as_str() {
                                    "KiB" => (size * 1024.0) as u64,
                                    "MiB" => (size * 1024.0 * 1024.0) as u64,
                                    "GiB" => (size * 1024.0 * 1024.0 * 1024.0) as u64,
                                    _ => size as u64,
                                };
                                total.store(size_bytes, Ordering::Relaxed);
                                
                                // Calculate downloaded based on percentage
                                let dl = (size_bytes as f64 * percent_str.as_str().parse::<f64>().unwrap_or(0.0) / 100.0) as u64;
                                downloaded.store(dl, Ordering::Relaxed);
                            }
                        }
                    }
                    
                    // Extract title
                    if let Some(caps) = re_title.captures(&line) {
                        if let Some(title_match) = caps.get(1) {
                            let mut t = title_clone.lock().unwrap();
                            *t = title_match.as_str().to_string();
                        }
                    }
                }
            });
        }
        
        // Process stderr for errors
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Print errors to console for debugging
                    if line.contains("ERROR:") || line.contains("Error:") {
                        eprintln!("{}", line.red());
                    } else if line.contains("WARNING:") || line.contains("Warning:") {
                        eprintln!("{}", line.yellow());
                    }
                }
            });
        }
        
        // Wait for the command to complete
        let status = child.wait().await
            .map_err(|e| AppError::DownloadError(format!("yt-dlp process error: {}", e)))?;
        
        // Abort the progress task
        progress_task.abort();
        
        if !status.success() {
            return Err(AppError::DownloadError(format!("yt-dlp exited with status: {}", status)));
        }
        
        // Get the title
        let title_str = {
            let t = title.lock().unwrap();
            t.clone()
        };
        
        // Update progress to 100% when complete
        percentage.store(100, Ordering::Relaxed);
        pb.set_position(100);
        pb.finish_with_message(format!("Download complete: {}", title_str));
        
        Ok(format!("Successfully downloaded: {}", title_str))
    }
    
    /// Get estimated download size without actually downloading
    pub async fn get_estimated_size(&self) -> Result<u64, AppError> {
        // Build command to get download size
        let mut cmd = Command::new("yt-dlp");
        
        cmd.arg("--print")
           .arg("filesize")
           .arg("--no-download")
           .arg("--no-playlist")
           .arg(&self.config.url);
        
        // Add format selection based on quality
        if let Some(quality) = &self.config.quality {
            match quality.as_str() {
                "480" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=480]/bestvideo[height<=480]+bestaudio/best");
                },
                "720" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=720]/bestvideo[height<=720]+bestaudio/best");
                },
                "1080" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=1080]/bestvideo[height<=1080]+bestaudio/best");
                },
                "2160" => {
                    cmd.arg("-f");
                    cmd.arg("best[height<=2160]/bestvideo[height<=2160]+bestaudio/best");
                },
                _ => {
                    // Default to 720p
                    cmd.arg("-f");
                    cmd.arg("best[height<=720]/bestvideo[height<=720]+bestaudio/best");
                }
            }
        } else if self.config.format == "mp3" {
            // For audio, use audio only format
            cmd.arg("-f").arg("bestaudio");
        }
        
        // Execute command
        let output = cmd.output().await
            .map_err(|e| AppError::DownloadError(format!("Failed to get download size: {}", e)))?;
        
        if !output.status.success() {
            return Err(AppError::DownloadError("Failed to get download size".to_string()));
        }
        
        // Parse the output
        let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        match size_str.parse::<u64>() {
            Ok(size) => Ok(size),
            Err(_) => Ok(0), // Default to 0 if unparseable
        }
    }
    
    /// List available formats for a video
    pub async fn list_formats(&self) -> Result<Vec<FormatInfo>, AppError> {
        // Try to get already fetched video info first
        if let Ok(video_info) = self.get_video_info().await {
            if !video_info.formats.is_empty() {
                return Ok(video_info.formats);
            }
        }
        
        // Build command to get formats
        let mut cmd = Command::new("yt-dlp");
        
        cmd.arg("--list-formats")
           .arg("--no-playlist")
           .arg(&self.config.url);
        
        // Execute command
        let output = cmd.output().await
            .map_err(|e| AppError::DownloadError(format!("Failed to list formats: {}", e)))?;
        
        if !output.status.success() {
            return Err(AppError::DownloadError("Failed to list formats".to_string()));
        }
        
        // Parse the output to extract formats
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut formats = Vec::new();
        
        // Simple regex to extract formats from the output
        let format_regex = Regex::new(r"(\w+)\s+(\w+)\s+(\d+x\d+|\d+p|audio only)\s+(.+)").unwrap();
        
        for line in output_str.lines() {
            if let Some(caps) = format_regex.captures(line) {
                let format_id = caps.get(1).map_or("", |m| m.as_str()).to_string();
                let ext = caps.get(2).map_or("", |m| m.as_str()).to_string();
                let resolution = caps.get(3).map_or("", |m| m.as_str()).to_string();
                let note = caps.get(4).map_or("", |m| m.as_str()).to_string();
                
                // Parse width and height from resolution
                let (width, height) = if resolution.contains("x") {
                    let parts: Vec<&str> = resolution.split('x').collect();
                    if parts.len() == 2 {
                        let w = parts[0].parse::<u32>().ok();
                        let h = parts[1].parse::<u32>().ok();
                        (w, h)
                    } else {
                        (None, None)
                    }
                } else if resolution.ends_with('p') {
                    let h = resolution.trim_end_matches('p').parse::<u32>().ok();
                    // Approximating width based on common aspect ratios
                    let w = h.map(|h| (h as f32 * 16.0 / 9.0) as u32);
                    (w, h)
                } else {
                    (None, None)
                };
                
                formats.push(FormatInfo {
                    format_id,
                    format_note: note,
                    ext,
                    width,
                    height,
                    file_size: None,
                });
            }
        }
        
        Ok(formats)
    }
}

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