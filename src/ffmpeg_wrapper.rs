// src/ffmpeg_wrapper.rs

use crate::error::AppError;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::{Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// FFmpeg wrapper for audio/video conversion
pub struct FFmpegWrapper {
    input_path: PathBuf,
    output_path: PathBuf,
    start_time: Option<String>,
    end_time: Option<String>,
    bitrate: Option<String>,
}


pub fn init() -> Result<(), crate::error::AppError> {
    // Check if ffmpeg is accessible
    match std::process::Command::new("ffmpeg")
        .arg("-version")
        .output() {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    println!("FFmpeg initialized: {}", version.lines().next().unwrap_or("Unknown version"));
                    Ok(())
                } else {
                    Err(crate::error::AppError::MissingDependency("FFmpeg returned error status".to_string()))
                }
            },
            Err(_) => Err(crate::error::AppError::MissingDependency("FFmpeg not found in PATH".to_string()))
        }
}

impl FFmpegWrapper {
    /// Create a new FFmpeg wrapper
    pub fn new(
        input_path: PathBuf,
        output_path: PathBuf,
        start_time: Option<String>,
        end_time: Option<String>,
        bitrate: Option<String>,
    ) -> Self {
        Self {
            input_path,
            output_path,
            start_time,
            end_time,
            bitrate,
        }
    }
    
    /// Convert a video file to MP3 audio
    pub async fn convert_to_mp3(&self) -> Result<(), AppError> {
        // Create a progress bar
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        // Build the ffmpeg command
        let mut cmd = Command::new("ffmpeg");
        
        // Add basic arguments
        cmd.arg("-i")
           .arg(&self.input_path)
           .arg("-vn"); // No video
        
        // Add start time if specified
        if let Some(start) = &self.start_time {
            cmd.arg("-ss").arg(start);
        }
        
        // Add end time if specified
        if let Some(end) = &self.end_time {
            cmd.arg("-to").arg(end);
        }
        
        // Set audio codec and quality
        cmd.arg("-acodec").arg("libmp3lame");
        
        // Add bitrate if specified
        if let Some(bitrate) = &self.bitrate {
            cmd.arg("-b:a").arg(bitrate);
        } else {
            cmd.arg("-b:a").arg("128k"); // Default bitrate
        }
        
        // Add output file
        cmd.arg(&self.output_path);
        
        // Add overwrite flag
        cmd.arg("-y");
        
        // Set up stdin, stdout, and stderr for capturing output
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        println!("{} {}", "Running ffmpeg command:".blue(), format!("{:?}", cmd).yellow());
        
        // Execute the command
        let mut child = cmd.spawn()
            .map_err(|e| AppError::DownloadError(format!("Failed to execute ffmpeg: {}", e)))?;
        
        // Set up progress tracking
        let percentage = Arc::new(AtomicU64::new(0));
        let duration_seconds = Arc::new(AtomicU64::new(0));
        let current_time = Arc::new(AtomicU64::new(0));
        
        // Create a clone of pb for the progress task
        let pb_clone = pb.clone();
        
        // Set up a timer for progress updates
        let progress_task = {
            let percentage = percentage.clone();
            
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    
                    let pct = percentage.load(Ordering::Relaxed);
                    pb_clone.set_position(pct);
                    
                    if pct >= 100 {
                        break;
                    }
                }
            })
        };
        
        // Process stderr to capture progress information (ffmpeg outputs progress to stderr)
        if let Some(stderr) = child.stderr.take() {
            let percentage = percentage.clone();
            let duration_seconds = duration_seconds.clone();
            let current_time = current_time.clone();
            
            // Create another clone of pb for the stderr handler
            let pb_stderr = pb.clone();
            
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                
                // Regex for extracting duration
                let duration_re = Regex::new(r"Duration: (\d+):(\d+):(\d+)").unwrap();
                
                // Regex for extracting current time
                let time_re = Regex::new(r"time=(\d+):(\d+):(\d+)").unwrap();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Process duration info
                    if let Some(caps) = duration_re.captures(&line) {
                        if let (Some(hours), Some(minutes), Some(seconds)) = 
                            (caps.get(1), caps.get(2), caps.get(3)) {
                            
                            let h: u64 = hours.as_str().parse().unwrap_or(0);
                            let m: u64 = minutes.as_str().parse().unwrap_or(0);
                            let s: u64 = seconds.as_str().parse().unwrap_or(0);
                            
                            let total_seconds = h * 3600 + m * 60 + s;
                            duration_seconds.store(total_seconds, Ordering::Relaxed);
                        }
                    }
                    
                    // Process time progress
                    if let Some(caps) = time_re.captures(&line) {
                        if let (Some(hours), Some(minutes), Some(seconds)) = 
                            (caps.get(1), caps.get(2), caps.get(3)) {
                            
                            let h: u64 = hours.as_str().parse().unwrap_or(0);
                            let m: u64 = minutes.as_str().parse().unwrap_or(0);
                            let s: u64 = seconds.as_str().parse().unwrap_or(0);
                            
                            let elapsed_seconds = h * 3600 + m * 60 + s;
                            current_time.store(elapsed_seconds, Ordering::Relaxed);
                            
                            // Calculate percentage
                            let total = duration_seconds.load(Ordering::Relaxed);
                            if total > 0 {
                                let pct = elapsed_seconds * 100 / total;
                                percentage.store(pct, Ordering::Relaxed);
                                
                                // Update progress bar message
                                pb_stderr.set_message(format!("Converting: {} / {} seconds", 
                                             elapsed_seconds,
                                             total));
                            }
                        }
                    }
                    
                    // Print errors and warnings to console
                    if line.contains("Error") || line.contains("error") {
                        eprintln!("{}", line.red());
                    } else if line.contains("Warning") || line.contains("warning") {
                        eprintln!("{}", line.yellow());
                    }
                }
            });
        }
        
        // Process stdout for any additional information
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Print stdout for debugging
                    println!("{}", line);
                }
            });
        }
        
        // Wait for the command to complete
        let status = child.wait().await
            .map_err(|e| AppError::DownloadError(format!("ffmpeg process error: {}", e)))?;
        
        // Abort the progress task
        progress_task.abort();
        
        if !status.success() {
            return Err(AppError::DownloadError(format!("ffmpeg exited with status: {}", status)));
        }
        
        // Update progress to 100% when complete
        percentage.store(100, Ordering::Relaxed);
        pb.set_position(100);
        pb.finish_with_message("Conversion complete");
        
        Ok(())
    }
    
    /// Get information about a media file
    pub async fn get_media_info(file_path: &Path) -> Result<MediaInfo, AppError> {
        // Build the ffprobe command
        let mut cmd = Command::new("ffprobe");
        
        cmd.arg("-v").arg("quiet")
           .arg("-print_format").arg("json")
           .arg("-show_format")
           .arg("-show_streams")
           .arg(file_path);
        
        // Execute the command
        let output = cmd.output().await
            .map_err(|e| AppError::DownloadError(format!("Failed to execute ffprobe: {}", e)))?;
        
        if !output.status.success() {
            return Err(AppError::DownloadError(format!("ffprobe exited with status: {}", output.status)));
        }
        
        // Parse the JSON output
        let json = String::from_utf8(output.stdout)
            .map_err(|e| AppError::DownloadError(format!("Invalid ffprobe output: {}", e)))?;
        
        let info: MediaInfo = serde_json::from_str(&json)
            .map_err(|e| AppError::DownloadError(format!("Failed to parse ffprobe output: {}", e)))?;
        
        Ok(info)
    }
}

/// Direct function to convert a file to MP3
pub async fn convert_to_mp3(
    input_path: &str,
    output_path: &str,
    bitrate: &str,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> Result<(), AppError> {
    let converter = FFmpegWrapper::new(
        PathBuf::from(input_path),
        PathBuf::from(output_path),
        start_time.map(String::from),
        end_time.map(String::from),
        Some(bitrate.to_string()),
    );
    
    converter.convert_to_mp3().await
}

/// Media file information
#[derive(Debug, serde::Deserialize)]
pub struct MediaInfo {
    pub format: Format,
    pub streams: Vec<Stream>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Format {
    pub filename: String,
    pub duration: Option<String>,
    pub size: Option<String>,
    pub bit_rate: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Stream {
    pub codec_type: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<String>,
    pub bit_rate: Option<String>,
}