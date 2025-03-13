// src/ytdlp_wrapper.rs

use crate::error::AppError;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::env;
use std::path::PathBuf;
use std::process::{Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

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

/// Wrapper for yt-dlp command-line tool
pub struct YtDlpWrapper {
    config: DownloadConfig,
    progress_callback: Option<ProgressCallback>,
    binary_path: String,  // Added to support custom binary path
}

impl YtDlpWrapper {
    /// Create a new wrapper with the given configuration
    pub fn new(
        config: DownloadConfig, 
        progress_callback: Option<ProgressCallback>
    ) -> Self {
        // Check for custom yt-dlp path in environment
        let binary_path = env::var("RUSTLOADER_YTDLP_PATH")
            .unwrap_or_else(|_| "yt-dlp".to_string());
            
        Self {
            config,
            progress_callback,
            binary_path,
        }
    }
    
    /// Check if yt-dlp is working correctly
    pub async fn check_ytdlp(&self) -> Result<(), AppError> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--version");
        
        match cmd.output().await {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    println!("{}: {}", "yt-dlp version".green(), version.trim());
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(AppError::MissingDependency(format!(
                        "yt-dlp returned error: {}", stderr.trim()
                    )))
                }
            },
            Err(e) => {
                Err(AppError::MissingDependency(format!(
                    "yt-dlp not found or not executable: {}. Please install yt-dlp with 'pip install yt-dlp'", e
                )))
            }
        }
    }
    
    /// Download video with progress tracking
    pub async fn download(&self) -> Result<String, AppError> {
        // Verify yt-dlp is working before starting
        self.check_ytdlp().await?;
        
        // Verify output directory exists
        if !self.config.output_dir.exists() {
            std::fs::create_dir_all(&self.config.output_dir)
                .map_err(|e| AppError::IoError(e))?;
            println!("Created output directory: {:?}", self.config.output_dir);
        }
        
        // Create a progress bar
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        // Build the yt-dlp command with all necessary arguments
        let mut cmd = Command::new(&self.binary_path);
        
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
                }
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
        
        // Execute the command with improved error handling
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                pb.finish_with_message("Failed to start download");
                return Err(AppError::DownloadError(format!(
                    "Failed to execute yt-dlp: {}. Make sure yt-dlp is installed correctly.", e
                )));
            }
        };
        
        // Set up Atomics for progress tracking
        let downloaded = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(100));
        let percentage = Arc::new(AtomicU64::new(0));
        let title = Arc::new(std::sync::Mutex::new(String::from("Unknown")));
        
        // Clone for progress callback
        let progress_callback = self.progress_callback.clone();
        
        // Set up a timer for progress updates
        let progress_task = {
            let downloaded = downloaded.clone();
            let total = total.clone();
            let percentage = percentage.clone();
            let pb = pb.clone();
            
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
                        if !callback(dl, tot) {
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
                    // Print errors to console with color coding
                    if line.contains("ERROR") || line.contains("Error") {
                        eprintln!("{}", line.red());
                    } else if line.contains("WARNING") || line.contains("Warning") {
                        eprintln!("{}", line.yellow());
                    } else {
                        // Print other stderr lines in normal color for debugging
                        eprintln!("{}", line);
                    }
                }
            });
        }
        
        // Wait for the command to complete
        let status = match child.wait().await {
            Ok(status) => status,
            Err(e) => {
                progress_task.abort();
                pb.finish_with_message("Download process error");
                return Err(AppError::DownloadError(format!("yt-dlp process error: {}", e)));
            }
        };
        
        // Abort the progress task
        progress_task.abort();
        
        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            
            // Provide helpful messages for common error codes
            let error_message = match exit_code {
                1 => "yt-dlp had some warnings but may have succeeded partially. Check the output directory.",
                2 => "yt-dlp was unable to download the video. The URL might be invalid or unsupported.",
                3 => "yt-dlp had a fatal error. Please check the error messages above.",
                _ => "yt-dlp exited with an unknown error code.",
            };
            
            return Err(AppError::DownloadError(format!(
                "yt-dlp exited with status: {} (code: {}). {}", status, exit_code, error_message
            )));
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