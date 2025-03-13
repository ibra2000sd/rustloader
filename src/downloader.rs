// src/downloader.rs

use crate::error::AppError;
use crate::ytdlp_wrapper::{YtDlpWrapper, DownloadConfig};
use crate::utils::initialize_download_dir;
use crate::counter::{check_daily_limit, increment_daily_count};
use crate::promo::DownloadPromo;
use colored::*;
use notify_rust::Notification;
use std::sync::Arc;
use crate::error::AppError;
use crate::ytdlp_wrapper::{YtDlpWrapper, DownloadConfig};
use crate::utils::initialize_download_dir;
use crate::counter::{check_daily_limit, increment_daily_count};
use crate::promo::DownloadPromo;
use colored::*;
use notify_rust::Notification;
use std::sync::Arc;

// Constants for free version limitations
const MAX_FREE_QUALITY: &str = "720";
const FREE_MP3_BITRATE: &str = "128K";

/// Structure for tracking download progress
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,    // bytes per second
    pub percentage: u64,
}

impl DownloadProgress {
    pub fn new() -> Self {
        Self {
            downloaded: 0,
            total: 0,
            speed: 0.0,
            percentage: 0,
        }
    }
    
    pub fn update(&mut self, downloaded: u64, total: u64, speed: f64) {
        self.downloaded = downloaded;
        self.total = total;
        self.speed = speed;
        
        if total > 0 {
            self.percentage = (downloaded * 100) / total;
        } else {
            self.percentage = 0;
        }
    }
    
    /// Format the progress as a string
    pub fn format(&self) -> String {
        let downloaded_str = format_bytes(self.downloaded);
        let total_str = if self.total > 0 {
            format_bytes(self.total)
        } else {
            "Unknown".to_string()
        };
        
        let speed_str = format_bytes_per_second(self.speed);
        
        format!("{} / {} ({:.1}%) at {}", 
                downloaded_str, 
                total_str, 
                self.percentage as f64, 
                speed_str)
    }
}

/// Format bytes in a human-readable format
fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    
    let bytes_f64 = bytes as f64;
    
    if bytes_f64 >= GB {
        format!("{:.2} GB", bytes_f64 / GB)
    } else if bytes_f64 >= MB {
        format!("{:.2} MB", bytes_f64 / MB)
    } else if bytes_f64 >= KB {
        format!("{:.2} KB", bytes_f64 / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Format bytes per second in a human-readable format
fn format_bytes_per_second(bytes_per_second: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_second as u64))
}

/// Download a video with free version limitations
pub async fn download_video_free<F>(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&str>,
    end_time: Option<&str>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<&str>,
    force_download: bool,
    bitrate: Option<&str>,
    progress_callback: Option<F>,
) -> Result<(), AppError>
where
    F: Fn(u64, u64) -> bool + Send + Sync + 'static,
{
    // Apply free version limitations
    // Fix type mismatch: make sure all arms return Option<String>
    let limited_quality = match quality {
        Some("1080") | Some("2160") => {
            println!("{}", "Free version limited to 720p. Upgrade to Pro for higher quality.".yellow());
            Some(MAX_FREE_QUALITY.to_string())
        },
        Some(q) => Some(q.to_string()),
        None => Some(MAX_FREE_QUALITY.to_string()),
    };
    
    // Apply free version audio quality limits
    let limited_bitrate = match format {
        "mp3" => Some(FREE_MP3_BITRATE.to_string()),
        _ => bitrate.map(|s| s.to_string()),
    };
    
    // Check daily download limits
    if !force_download {
        check_daily_limit()?;
    }
    
    // Create progress callback wrapper if provided
    let progress_arc = progress_callback.map(|cb| {
        let cb_arc: Arc<dyn Fn(u64, u64) -> bool + Send + Sync> = Arc::new(move |downloaded, total| {
            cb(downloaded, total)
        });
        cb_arc
    });
    
    // Initialize download dir
    let download_dir = initialize_download_dir(
        output_dir,
        "rustloader",
        if format == "mp3" { "audio" } else { "video" },
    )?;
    
    // Create download configuration
    let config = DownloadConfig {
        url: url.to_string(),
        quality: limited_quality,  // Now this is Option<String>, matching DownloadConfig's field type
        format: format.to_string(),
        start_time: start_time.map(|s| s.to_string()),
        end_time: end_time.map(|s| s.to_string()),
        use_playlist,
        download_subtitles,
        output_dir: download_dir,
        bitrate: limited_bitrate,
    };
    
    // Create downloader
    let downloader = YtDlpWrapper::new(config, progress_arc);
    
    // Display a promotional message if in free version
    let promo = DownloadPromo::new();
    println!("\n{}\n", promo.get_random_download_message().bright_yellow());
    
    // Execute the download
    match downloader.download().await {
        Ok(message) => {
            // Increment the daily download counter
            if !force_download {
                increment_daily_count()?;
            }
            
            // Send a desktop notification
            if let Err(e) = Notification::new()
                .summary("Download Complete")
                .body(&message)
                .show() {
                println!("{}: {}", "Failed to show notification".yellow(), e);
            }
            
            // Display completion message
            println!("{}", message.green());
            println!("{}", promo.get_random_completion_message().bright_yellow());
            
            Ok(())
        },
        Err(e) => Err(e),
    }
}

/// Download a video with pro version capabilities
pub async fn download_video_pro<F>(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&str>,
    end_time: Option<&str>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<&str>,
    _force_download: bool, // Prefix with _ to avoid unused variable warning
    bitrate: Option<&str>,
    progress_callback: Option<F>,
) -> Result<(), AppError>
where
    F: Fn(u64, u64) -> bool + Send + Sync + 'static,
{
    // Create progress callback wrapper if provided
    let progress_arc = progress_callback.map(|cb| {
        let cb_arc: Arc<dyn Fn(u64, u64) -> bool + Send + Sync> = Arc::new(move |downloaded, total| {
            cb(downloaded, total)
        });
        cb_arc
    });
    
    // Initialize download dir
    let download_dir = initialize_download_dir(
        output_dir,
        "rustloader",
        if format == "mp3" { "audio" } else { "video" },
    )?;
    
    // Create download configuration with pro features enabled - fixed type consistency
    let config = DownloadConfig {
        url: url.to_string(),
        quality: quality.map(|s| s.to_string()),  // Ensure we have Option<String>
        format: format.to_string(),
        start_time: start_time.map(|s| s.to_string()),
        end_time: end_time.map(|s| s.to_string()),
        use_playlist,
        download_subtitles,
        output_dir: download_dir,
        bitrate: bitrate.map(|s| s.to_string()),
    };
    
    // Create downloader
    let downloader = YtDlpWrapper::new(config, progress_arc);
    
    // Execute the download
    match downloader.download().await {
        Ok(message) => {
            // Send a desktop notification
            if let Err(e) = Notification::new()
                .summary("Download Complete")
                .body(&message)
                .show() {
                println!("{}: {}", "Failed to show notification".yellow(), e);
            }
            
            // Display completion message
            println!("{}", message.green());
            
            Ok(())
        },
        Err(e) => Err(e),
    }
}