// src/youtube_dl_wrapper.rs
//
// Rewritten to use Rust-native libraries instead of external yt-dlp tool

use crate::error::AppError;
use crate::security;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use async_trait::async_trait;
use reqwest::Client;
use rustube::{Video, VideoFetcher};
use url::Url;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use indicatif::{ProgressBar, ProgressStyle};
use mime_guess::from_path;
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

/// Wrapper for native Rust-based video/audio downloading
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
        
        // Detect platform and use appropriate downloader
        match self.detect_platform() {
            PlatformType::YouTube => self.download_youtube().await,
            PlatformType::Vimeo => self.download_vimeo().await,
            PlatformType::Other => self.download_generic().await,
        }
    }
    
    /// Download from YouTube using rustube
    async fn download_youtube(&self) -> Result<String, AppError> {
        println!("Using native Rust YouTube downloader");
        
        // Parse the video ID from the URL
        let url = Url::parse(&self.config.url)
            .map_err(|_| AppError::ValidationError("Invalid YouTube URL".to_string()))?;
            
        // Create video fetcher
        let video = match VideoFetcher::from_url(&self.config.url) {
            Ok(fetcher) => {
                fetcher.fetch()
                    .await
                    .map_err(|e| AppError::DownloadError(format!("Failed to fetch video: {}", e)))?
            },
            Err(e) => {
                return Err(AppError::DownloadError(format!("Failed to create video fetcher: {}", e)))
            }
        };
        
        // Get the best quality based on config
        let stream = self.select_stream(&video)?;
        
        // Get video title for filename
        let title = video.title().to_string();
        let sanitized_title = sanitize_filename(&title);
        
        // Create output filename
        let extension = match self.config.format.as_str() {
            "mp3" => "mp3",
            "m4a" => "m4a",
            _ => "mp4",
        };
        
        let output_filename = format!("{}.{}", sanitized_title, extension);
        let output_path = self.config.output_dir.join(&output_filename);
        
        // Create progress bar
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        // Start download
        println!("Downloading to {}", output_path.display());
        pb.set_message("Starting download...");
        
        // Stream information
        let stream_url = stream.signature_cipher.url.clone();
        
        // Create destination file
        let mut file = File::create(&output_path)
            .await
            .map_err(|e| AppError::IoError(e))?;
            
        // Download the stream
        let response = self.client.get(&stream_url)
            .send()
            .await
            .map_err(|e| AppError::DownloadError(format!("Failed to start download: {}", e)))?;
            
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0;
        let mut stream = response.bytes_stream();
        
        // Create wrapper for progress callback
        let progress_arc = Arc::clone(&self.progress);
        let progress_callback = self.progress_callback.clone();
        
        // Process the download stream
        use futures_util::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| AppError::DownloadError(format!("Error during download: {}", e)))?;
                
            // Write the chunk to file
            file.write_all(&chunk)
                .await
                .map_err(|e| AppError::IoError(e))?;
                
            // Update progress
            downloaded += chunk.len() as u64;
            let percentage = if total_size > 0 {
                (downloaded * 100) / total_size
            } else {
                0
            };
            
            // Update progress bar
            pb.set_position(percentage);
            pb.set_message(format!("Downloaded: {} / {}", 
                                   format_bytes(downloaded), 
                                   format_bytes(total_size)));
                                   
            // Update internal progress tracker
            {
                let mut current = progress_arc.lock().await;
                *current = (downloaded, total_size);
            }
            
            // Call progress callback if provided
            if let Some(callback) = &progress_callback {
                if !callback(downloaded, total_size) {
                    // Callback returned false - abort download
                    break;
                }
            }
        }
        
        // Finish download
        file.flush().await.map_err(|e| AppError::IoError(e))?;
        pb.finish_with_message(format!("Downloaded {} to {}", 
                                     format_bytes(total_size), 
                                     output_path.display()));
        
        // If format is mp3 and download was mp4, convert to mp3
        if self.config.format == "mp3" && extension != "mp3" {
            println!("Converting to MP3...");
            self.convert_to_mp3(&output_path).await?;
        }
        
        // If subtitles requested, download them
        if self.config.download_subtitles {
            self.download_subtitles(&video, &sanitized_title).await?;
        }
        
        Ok(format!("Successfully downloaded: {}", title))
    }
    
    /// Select the best stream based on config
    fn select_stream<'a>(&self, video: &'a Video) -> Result<&'a rustube::Stream, AppError> {
        let streams = video.streams();
        
        // If MP3/audio is requested, prefer audio streams
        if self.config.format == "mp3" {
            let audio_streams = streams.iter()
                .filter(|s| s.includes_audio_track && !s.includes_video_track)
                .collect::<Vec<_>>();
                
            if !audio_streams.is_empty() {
                // Find highest quality audio
                return Ok(audio_streams.iter()
                    .max_by_key(|s| s.quality_label.clone().unwrap_or_default())
                    .unwrap());
            }
            
            // If no audio-only streams, fall back to video with audio
            let streams_with_audio = streams.iter()
                .filter(|s| s.includes_audio_track)
                .collect::<Vec<_>>();
                
            if !streams_with_audio.is_empty() {
                return Ok(streams_with_audio.iter()
                    .max_by_key(|s| s.quality_label.clone().unwrap_or_default())
                    .unwrap());
            }
            
            return Err(AppError::DownloadError("No audio streams found".to_string()));
        }
        
        // For video, consider quality restrictions
        let max_height = match self.config.quality.as_deref() {
            Some("480") => 480,
            Some("720") => 720,
            Some("1080") => 1080,
            Some("2160") => 2160,
            _ => 720, // Default to 720p
        };
        
        // Find streams that include video track
        let video_streams = streams.iter()
            .filter(|s| s.includes_video_track)
            .collect::<Vec<_>>();
            
        if video_streams.is_empty() {
            return Err(AppError::DownloadError("No video streams found".to_string()));
        }
        
        // Extract height from quality label, e.g. "720p" -> 720
        let parse_height = |label: &str| -> Option<u32> {
            let re = Regex::new(r"(\d+)p").ok()?;
            re.captures(label)?.get(1)?.as_str().parse::<u32>().ok()
        };
        
        // Filter by max height
        let filtered_streams = video_streams.iter()
            .filter(|s| {
                if let Some(label) = &s.quality_label {
                    if let Some(height) = parse_height(label) {
                        return height <= max_height;
                    }
                }
                true // Include if we can't parse height
            })
            .collect::<Vec<_>>();
            
        if filtered_streams.is_empty() {
            // If no streams match our criteria, just use the best available
            return Ok(video_streams.iter()
                .max_by_key(|s| s.quality_label.clone().unwrap_or_default())
                .unwrap());
        }
        
        // Sort by quality (descending) and pick the best
        Ok(filtered_streams.iter()
            .max_by_key(|s| s.quality_label.clone().unwrap_or_default())
            .unwrap())
    }
    
    /// Download video subtitles
    async fn download_subtitles(&self, video: &Video, title: &str) -> Result<(), AppError> {
        println!("Downloading subtitles...");
        
        // First try to get closed captions
        if let Some(captions) = video.captions() {
            // Find English or just first available
            let caption = captions.iter()
                .find(|c| c.language_code == "en")
                .or_else(|| captions.iter().next());
                
            if let Some(caption) = caption {
                // Download subtitle
                let subs_url = &caption.base_url;
                let response = self.client.get(subs_url)
                    .send()
                    .await
                    .map_err(|e| AppError::DownloadError(format!("Failed to download subtitles: {}", e)))?;
                    
                let content = response.text()
                    .await
                    .map_err(|e| AppError::DownloadError(format!("Failed to read subtitles: {}", e)))?;
                    
                // Save to file
                let subs_path = self.config.output_dir.join(format!("{}.{}", title, "srt"));
                let mut file = File::create(&subs_path)
                    .await
                    .map_err(|e| AppError::IoError(e))?;
                    
                file.write_all(content.as_bytes())
                    .await
                    .map_err(|e| AppError::IoError(e))?;
                    
                println!("Subtitles saved to {}", subs_path.display());
                return Ok(());
            }
        }
        
        println!("No subtitles found for this video");
        Ok(())
    }
    
    /// Convert video to MP3
    async fn convert_to_mp3(&self, video_path: &Path) -> Result<(), AppError> {
        use tokio::process::Command;
        
        // Get output file path
        let parent = video_path.parent().unwrap_or(Path::new("."));
        let stem = video_path.file_stem().unwrap_or_default().to_string_lossy();
        let mp3_path = parent.join(format!("{}.mp3", stem));
        
        // Prepare ffmpeg command
        let bitrate = self.config.bitrate.as_deref().unwrap_or("128k");
        
        println!("Converting to MP3 with bitrate: {}", bitrate);
        
        // Use our Rust ffmpeg wrapper for conversion
        crate::ffmpeg_wrapper::convert_to_audio(
            video_path.to_str().unwrap(),
            mp3_path.to_str().unwrap(),
            bitrate,
            self.config.start_time.as_deref(),
            self.config.end_time.as_deref(),
        ).await?;
        
        // Remove the original video file if conversion successful
        if mp3_path.exists() {
            tokio::fs::remove_file(video_path).await
                .map_err(|e| AppError::IoError(e))?;
            println!("Converted to MP3 and removed original video file");
        }
        
        Ok(())
    }
    
    /// Vimeo download implementation
    async fn download_vimeo(&self) -> Result<String, AppError> {
        // Since we don't have a native Rust library for Vimeo yet,
        // implement a basic downloader using direct HTTP requests
        
        println!("Using native Rust Vimeo downloader");
        
        // First, fetch the page to get the video configuration
        let response = self.client.get(&self.config.url)
            .send()
            .await
            .map_err(|e| AppError::DownloadError(format!("Failed to fetch Vimeo page: {}", e)))?;
            
        let html = response.text()
            .await
            .map_err(|e| AppError::DownloadError(format!("Failed to read Vimeo page: {}", e)))?;
            
        // Extract video information using regex
        let title = extract_vimeo_title(&html)
            .ok_or_else(|| AppError::DownloadError("Could not extract Vimeo video title".to_string()))?;
            
        let config_url = extract_vimeo_config_url(&html)
            .ok_or_else(|| AppError::DownloadError("Could not extract Vimeo config URL".to_string()))?;
            
        // Fetch video configuration
        let config_response = self.client.get(&config_url)
            .send()
            .await
            .map_err(|e| AppError::DownloadError(format!("Failed to fetch Vimeo config: {}", e)))?;
            
        let config_json = config_response.text()
            .await
            .map_err(|e| AppError::DownloadError(format!("Failed to read Vimeo config: {}", e)))?;
            
        // Parse config to get video URLs
        let video_url = extract_vimeo_video_url(&config_json, self.config.quality.as_deref())
            .ok_or_else(|| AppError::DownloadError("Could not extract Vimeo video URL".to_string()))?;
            
        // Sanitize the title for use as filename
        let sanitized_title = sanitize_filename(&title);
        
        // Create output filename
        let extension = match self.config.format.as_str() {
            "mp3" => "mp3",
            _ => "mp4",
        };
        
        let output_filename = format!("{}.{}", sanitized_title, extension);
        let output_path = self.config.output_dir.join(&output_filename);
        
        // Create progress bar
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        // Start download
        println!("Downloading to {}", output_path.display());
        pb.set_message("Starting download...");
        
        // Create destination file
        let mut file = File::create(&output_path)
            .await
            .map_err(|e| AppError::IoError(e))?;
            
        // Download the video
        let response = self.client.get(&video_url)
            .send()
            .await
            .map_err(|e| AppError::DownloadError(format!("Failed to start download: {}", e)))?;
            
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0;
        let mut stream = response.bytes_stream();
        
        // Create wrapper for progress callback
        let progress_arc = Arc::clone(&self.progress);
        let progress_callback = self.progress_callback.clone();
        
        // Process the download stream
        use futures_util::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| AppError::DownloadError(format!("Error during download: {}", e)))?;
                
            // Write the chunk to file
            file.write_all(&chunk)
                .await
                .map_err(|e| AppError::IoError(e))?;
                
            // Update progress
            downloaded += chunk.len() as u64;
            let percentage = if total_size > 0 {
                (downloaded * 100) / total_size
            } else {
                0
            };
            
            // Update progress bar
            pb.set_position(percentage);
            pb.set_message(format!("Downloaded: {} / {}", 
                                   format_bytes(downloaded), 
                                   format_bytes(total_size)));
                                   
            // Update internal progress tracker
            {
                let mut current = progress_arc.lock().await;
                *current = (downloaded, total_size);
            }
            
            // Call progress callback if provided
            if let Some(callback) = &progress_callback {
                if !callback(downloaded, total_size) {
                    // Callback returned false - abort download
                    break;
                }
            }
        }
        
        // Finish download
        file.flush().await.map_err(|e| AppError::IoError(e))?;
        pb.finish_with_message(format!("Downloaded {} to {}", 
                                     format_bytes(total_size), 
                                     output_path.display()));
        
        // If format is mp3 and download was mp4, convert to mp3
        if self.config.format == "mp3" && extension != "mp3" {
            println!("Converting to MP3...");
            self.convert_to_mp3(&output_path).await?;
        }
        
        Ok(format!("Successfully downloaded: {}", title))
    }
    
    /// Generic download implementation for other platforms
    async fn download_generic(&self) -> Result<String, AppError> {
        // For sites we don't have specific support for yet,
        // implement a basic direct downloader
        
        println!("Using generic downloader");
        
        // Get filename from the URL
        let url = Url::parse(&self.config.url)
            .map_err(|_| AppError::ValidationError("Invalid URL".to_string()))?;
            
        let path = url.path();
        let filename = path.split('/').last().unwrap_or("video");
        
        // Create a sanitized filename
        let sanitized_filename = sanitize_filename(filename);
        
        // Create output filename
        let extension = match self.config.format.as_str() {
            "mp3" => "mp3",
            _ => "mp4",
        };
        
        let output_filename = format!("{}.{}", sanitized_filename, extension);
        let output_path = self.config.output_dir.join(&output_filename);
        
        // Create progress bar
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        // Start download
        println!("Downloading to {}", output_path.display());
        pb.set_message("Starting download...");
        
        // Try to download directly
        let response = self.client.get(&self.config.url)
            .send()
            .await
            .map_err(|e| AppError::DownloadError(format!("Failed to start download: {}", e)))?;
            
        // Check if content type is video/audio
        let content_type = response.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
            
        if !content_type.contains("video/") && 
           !content_type.contains("audio/") && 
           !content_type.contains("application/octet-stream") {
            return Err(AppError::DownloadError(
                format!("URL does not point to a media file. Content-Type: {}", content_type)
            ));
        }
        
        // Create destination file
        let mut file = File::create(&output_path)
            .await
            .map_err(|e| AppError::IoError(e))?;
            
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0;
        let mut stream = response.bytes_stream();
        
        // Create wrapper for progress callback
        let progress_arc = Arc::clone(&self.progress);
        let progress_callback = self.progress_callback.clone();
        
        // Process the download stream
        use futures_util::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| AppError::DownloadError(format!("Error during download: {}", e)))?;
                
            // Write the chunk to file
            file.write_all(&chunk)
                .await
                .map_err(|e| AppError::IoError(e))?;
                
            // Update progress
            downloaded += chunk.len() as u64;
            let percentage = if total_size > 0 {
                (downloaded * 100) / total_size
            } else {
                0
            };
            
            // Update progress bar
            pb.set_position(percentage);
            pb.set_message(format!("Downloaded: {} / {}", 
                                   format_bytes(downloaded), 
                                   format_bytes(total_size)));
                                   
            // Update internal progress tracker
            {
                let mut current = progress_arc.lock().await;
                *current = (downloaded, total_size);
            }
            
            // Call progress callback if provided
            if let Some(callback) = &progress_callback {
                if !callback(downloaded, total_size) {
                    // Callback returned false - abort download
                    break;
                }
            }
        }
        
        // Finish download
        file.flush().await.map_err(|e| AppError::IoError(e))?;
        pb.finish_with_message(format!("Downloaded {} to {}", 
                                     format_bytes(total_size), 
                                     output_path.display()));
        
        // If format is mp3 and download was mp4, convert to mp3
        if self.config.format == "mp3" && extension != "mp3" {
            println!("Converting to MP3...");
            self.convert_to_mp3(&output_path).await?;
        }
        
        Ok(format!("Successfully downloaded: {}", sanitized_filename))
    }
    
    /// Get current progress
    pub async fn get_progress(&self) -> (u64, u64) {
        *self.progress.lock().await
    }
}

// Helper functions

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

/// Extract Vimeo video title from HTML
fn extract_vimeo_title(html: &str) -> Option<String> {
    let re = Regex::new(r#"<title>(.*?)</title>"#).ok()?;
    let cap = re.captures(html)?;
    
    let title = cap.get(1)?.as_str().to_string();
    
    // Remove " on Vimeo" suffix if present
    let title = title.replace(" on Vimeo", "");
    
    Some(title)
}

/// Extract Vimeo config URL from HTML
fn extract_vimeo_config_url(html: &str) -> Option<String> {
    let re = Regex::new(r#"data-config-url="(.*?)""#).ok()?;
    let cap = re.captures(html)?;
    
    let url = cap.get(1)?.as_str().to_string();
    
    // HTML unescape
    let url = html_escape::decode_html_entities(&url).to_string();
    
    Some(url)
}

/// Extract Vimeo video URL from config JSON
fn extract_vimeo_video_url(json: &str, quality: Option<&str>) -> Option<String> {
    #[derive(Deserialize)]
    struct VimeoConfig {
        request: VimeoRequest,
    }
    
    #[derive(Deserialize)]
    struct VimeoRequest {
        files: VimeoFiles,
    }
    
    #[derive(Deserialize)]
    struct VimeoFiles {
        progressive: Vec<VimeoFile>,
    }
    
    #[derive(Deserialize)]
    struct VimeoFile {
        url: String,
        quality: String,
        height: Option<u32>,
        width: Option<u32>,
    }
    
    // Parse the JSON
    let config: VimeoConfig = serde_json::from_str(json).ok()?;
    
    // Get the requested max height
    let max_height = match quality {
        Some("480") => 480,
        Some("720") => 720,
        Some("1080") => 1080,
        Some("2160") => 2160,
        _ => 720, // Default to 720p
    };
    
    // Filter and get the best quality URL
    let files = &config.request.files.progressive;
    
    // First try to find a file matching the requested quality
    let matching_files: Vec<&VimeoFile> = files.iter()
        .filter(|f| {
            if let Some(height) = f.height {
                height <= max_height
            } else {
                true // Include if we can't determine height
            }
        })
        .collect();
        
    if !matching_files.is_empty() {
        // Sort by quality (descending) and pick the best
        let best_file = matching_files.iter()
            .max_by_key(|f| f.height.unwrap_or(0))
            .unwrap();
            
        return Some(best_file.url.clone());
    }
    
    // If no matching files, just return the best available
    let best_file = files.iter()
        .max_by_key(|f| f.height.unwrap_or(0))
        .unwrap();
        
    Some(best_file.url.clone())
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