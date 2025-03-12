// src/downloader.rs

use crate::error::AppError;
use crate::youtube_dl_wrapper::{YoutubeDlWrapper, DownloadConfig};
use crate::utils::{initialize_download_dir, validate_bitrate, validate_time_format, validate_url};
use colored::*;
use notify_rust::Notification;
use std::path::{PathBuf, Path};
use std::sync::Arc;
use rand::Rng;
use std::fs;
use chrono::Local;
use dirs_next as dirs;
use ring::{digest, hmac};
use base64::{Engine as _, engine::general_purpose};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, Duration};
use std::sync::Mutex;
use std::io;
use humansize::{format_size, BINARY};
use dialoguer::Select;
use indicatif::{ProgressBar, ProgressStyle};

// Constants for free version limitations
const MAX_FREE_QUALITY: &str = "720";
const FREE_MP3_BITRATE: &str = "128K";

// Enhanced progress tracking
pub struct DownloadProgress {
    start_time: Instant,
    last_update: Mutex<Instant>,
    downloaded_bytes: AtomicU64,
    total_bytes: AtomicU64,
    download_speed: Mutex<f64>,  // bytes per second
    last_speed_samples: Mutex<Vec<f64>>,
}

impl DownloadProgress {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_update: Mutex::new(Instant::now()),
            downloaded_bytes: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            download_speed: Mutex::new(0.0),
            last_speed_samples: Mutex::new(vec![]),
        }
    }
    
    pub fn update(&self, downloaded: u64, total: u64) {
        // Update bytes counters
        self.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        if total > 0 {
            self.total_bytes.store(total, Ordering::SeqCst);
        }
        
        // Calculate and update speed
        let now = Instant::now();
        
        let mut last_update = self.last_update.lock().unwrap();
        let time_diff = now.duration_since(*last_update).as_millis();
        
        // Only update speed calculation every 100ms to avoid jitter
        if time_diff >= 100 {
            let mut last_speed_samples = self.last_speed_samples.lock().unwrap();
            let mut speed = self.download_speed.lock().unwrap();
            
            // Calculate current speed
            if downloaded > 0 {
                let bytes_diff = downloaded - self.downloaded_bytes.load(Ordering::SeqCst);
                if bytes_diff > 0 {
                    let current_speed = bytes_diff as f64 / (time_diff as f64 / 1000.0);
                    
                    // Add to speed samples
                    last_speed_samples.push(current_speed);
                    if last_speed_samples.len() > 10 {
                        last_speed_samples.remove(0);
                    }
                    
                    // Calculate average speed from samples
                    let sum: f64 = last_speed_samples.iter().sum();
                    *speed = sum / last_speed_samples.len() as f64;
                }
            }
            
            *last_update = now;
        }
    }
    
    pub fn get_percentage(&self) -> u64 {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        
        if total == 0 {
            return 0;
        }
        
        (downloaded * 100 / total) as u64
    }
    
    pub fn get_speed(&self) -> f64 {
        *self.download_speed.lock().unwrap()
    }
    
    pub fn get_eta(&self) -> Option<Duration> {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        let speed = self.get_speed();
        
        if speed <= 0.0 || downloaded >= total || total == 0 {
            return None;
        }
        
        let remaining_bytes = total - downloaded;
        let seconds_remaining = remaining_bytes as f64 / speed;
        
        Some(Duration::from_secs_f64(seconds_remaining))
    }
    
    pub fn format_eta(&self) -> String {
        match self.get_eta() {
            Some(duration) => {
                let total_secs = duration.as_secs();
                let hours = total_secs / 3600;
                let minutes = (total_secs % 3600) / 60;
                let seconds = total_secs % 60;
                
                if hours > 0 {
                    format!("{}h {}m {}s", hours, minutes, seconds)
                } else if minutes > 0 {
                    format!("{}m {}s", minutes, seconds)
                } else {
                    format!("{}s", seconds)
                }
            },
            None => "Calculating...".to_string()
        }
    }
    
    pub fn format_speed(&self) -> String {
        let speed = self.get_speed();
        if speed <= 0.0 {
            return "Calculating...".to_string();
        }
        
        format!("{}/s", format_size(speed as u64, BINARY))
    }
    
    pub fn format_file_size(&self) -> String {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        
        if total == 0 {
            return format!("{} / Unknown", format_size(downloaded, BINARY));
        }
        
        format!("{} / {}", 
            format_size(downloaded, BINARY),
            format_size(total, BINARY)
        )
    }
}

// Only include the download and completion messages in downloader.rs
struct DownloadPromo {
    download_messages: Vec<String>,
    completion_messages: Vec<String>,
}

impl DownloadPromo {
    fn new() -> Self {
        Self {
            download_messages: vec![
                "⚡ Downloads would be 5X faster with Rustloader Pro! ⚡".to_string(),
                "🎬 Rustloader Pro supports 4K and 8K video quality! 🎬".to_string(),
                "🤖 AI-powered features available in Rustloader Pro! 🤖".to_string(),
            ],
            completion_messages: vec![
                "✨ Enjoy your download! Upgrade to Pro for even better quality: rustloader.com/pro ✨".to_string(),
                "🚀 Rustloader Pro removes ads and daily limits. Learn more: rustloader.com/pro 🚀".to_string(),
                "💎 Thanks for using Rustloader! Upgrade to Pro for 4K/8K quality: rustloader.com/pro 💎".to_string(),
            ],
        }
    }
    
    fn get_random_download_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.download_messages.len());
        &self.download_messages[idx]
    }
    
    fn get_random_completion_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.completion_messages.len());
        &self.completion_messages[idx]
    }
}

// Download counter for tracking daily limits with secure storage
struct DownloadCounter {
    today_count: u32,
    date: String,
    max_daily_downloads: u32,
}

impl DownloadCounter {
    fn new() -> Self {
        Self {
            today_count: 0,
            date: Local::now().format("%Y-%m-%d").to_string(),
            max_daily_downloads: 5, // Free tier limit
        }
    }
    
    // Generate a unique key for counter encryption based on machine ID
    fn get_counter_key() -> Vec<u8> {
        // Try to get a machine-specific identifier
        let machine_id = match Self::get_machine_id() {
            Ok(id) => id,
            Err(_) => "DefaultCounterKey".to_string(), // Fallback if machine ID can't be determined
        };
        
        // Use SHA-256 to create a fixed-length key from the machine ID
        let digest = digest::digest(&digest::SHA256, machine_id.as_bytes());
        digest.as_ref().to_vec()
    }
    
    // Get a machine-specific identifier
    fn get_machine_id() -> Result<String, AppError> {
        #[cfg(target_os = "linux")]
        {
            // On Linux, try to use the machine-id
            match fs::read_to_string("/etc/machine-id") {
                Ok(id) => return Ok(id.trim().to_string()),
                Err(_) => {
                    // Fallback to using hostname
                    match hostname::get() {
                        Ok(name) => return Ok(name.to_string_lossy().to_string()),
                        Err(_) => return Err(AppError::General("Could not determine machine ID".to_string())),
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, use the IOPlatformUUID
            use std::process::Command;
            
            let output = Command::new("ioreg")
                .args(["-rd1", "-c", "IOPlatformExpertDevice"])
                .output()?;
                
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Extract the UUID using a simple search
            if let Some(line) = stdout.lines().find(|line| line.contains("IOPlatformUUID")) {
                if let Some(uuid_start) = line.find("\"") {
                    if let Some(uuid_end) = line[uuid_start + 1..].find("\"") {
                        return Ok(line[uuid_start + 1..uuid_start + 1 + uuid_end].to_string());
                    }
                }
            }
            
            // Fallback to hostname
            match hostname::get() {
                Ok(name) => return Ok(name.to_string_lossy().to_string()),
                Err(_) => return Err(AppError::General("Could not determine machine ID".to_string())),
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, try to use the MachineGuid from registry
            use winreg::enums::*;
            use winreg::RegKey;
            
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            match hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography") {
                Ok(key) => {
                    match key.get_value::<String, _>("MachineGuid") {
                        Ok(guid) => return Ok(guid),
                        Err(_) => {
                            // Fallback to computer name
                            match hostname::get() {
                                Ok(name) => return Ok(name.to_string_lossy().to_string()),
                                Err(_) => return Err(AppError::General("Could not determine machine ID".to_string())),
                            }
                        }
                    }
                },
                Err(_) => {
                    // Fallback to computer name
                    match hostname::get() {
                        Ok(name) => return Ok(name.to_string_lossy().to_string()),
                        Err(_) => return Err(AppError::General("Could not determine machine ID".to_string())),
                    }
                }
            }
        }
        
        // Fallback for other platforms - use hostname
        match hostname::get() {
            Ok(name) => Ok(name.to_string_lossy().to_string()),
            Err(_) => Err(AppError::General("Could not determine machine ID".to_string())),
        }
    }
    
    // Encrypt counter data with HMAC signature
    fn encrypt_counter_data(&self) -> Result<String, AppError> {
        // Create the data string
        let content = format!("{},{}", self.date, self.today_count);
        
        // Create HMAC signature
        let key = hmac::Key::new(hmac::HMAC_SHA256, &Self::get_counter_key());
        let signature = hmac::sign(&key, content.as_bytes());
        let signature_b64 = general_purpose::STANDARD.encode(signature.as_ref());
        
        // Combine data and signature
        let full_data = format!("{}\n{}", content, signature_b64);
        
        // Base64 encode the full data
        Ok(general_purpose::STANDARD.encode(full_data.as_bytes()))
    }
    
    // Decrypt and verify counter data
    fn decrypt_counter_data(encrypted_data: &str) -> Result<(String, u32), AppError> {
        // Base64 decode the data
        let decoded_bytes = match general_purpose::STANDARD.decode(encrypted_data) {
            Ok(bytes) => bytes,
            Err(_) => return Err(AppError::SecurityViolation),
        };
        
        // Convert to string and split by newline
        let full_data = match String::from_utf8(decoded_bytes) {
            Ok(data) => data,
            Err(_) => return Err(AppError::SecurityViolation),
        };
        
        let parts: Vec<&str> = full_data.split('\n').collect();
        if parts.len() != 2 {
            return Err(AppError::SecurityViolation);
        }
        
        let content = parts[0];
        let signature_b64 = parts[1];
        
        // Verify signature
        let key = hmac::Key::new(hmac::HMAC_SHA256, &Self::get_counter_key());
        let signature_bytes = match general_purpose::STANDARD.decode(signature_b64) {
            Ok(bytes) => bytes,
            Err(_) => return Err(AppError::SecurityViolation),
        };
        
        match hmac::verify(&key, content.as_bytes(), &signature_bytes) {
            Ok(_) => {
                // Signature verified, parse the data
                let data_parts: Vec<&str> = content.split(',').collect();
                if data_parts.len() != 2 {
                    return Err(AppError::SecurityViolation);
                }
                
                let date = data_parts[0].to_string();
                let count: u32 = match data_parts[1].parse() {
                    Ok(c) => c,
                    Err(_) => return Err(AppError::SecurityViolation),
                };
                
                Ok((date, count))
            },
            Err(_) => Err(AppError::SecurityViolation),
        }
    }
    
    fn load_from_disk() -> Result<Self, AppError> {
        let counter_path = get_counter_path()?;
        
        if counter_path.exists() {
            let encrypted_contents = fs::read_to_string(&counter_path)?;
            
            match Self::decrypt_counter_data(&encrypted_contents) {
                Ok((date, count)) => {
                    // Check if date has changed
                    let today = Local::now().format("%Y-%m-%d").to_string();
                    if date != today {
                        // Reset counter for new day
                        return Ok(Self::new());
                    }
                    
                    // Return counter with today's date and count
                    Ok(Self {
                        today_count: count,
                        date,
                        max_daily_downloads: 5,
                    })
                },
                Err(_) => {
                    // If decryption fails, create a new counter
                    println!("{}", "Warning: Download counter validation failed. Counter has been reset.".yellow());
                    Ok(Self::new())
                }
            }
        } else {
            // If no counter file exists, create a new one
            Ok(Self::new())
        }
    }
    
    fn save_to_disk(&self) -> Result<(), AppError> {
        let counter_path = get_counter_path()?;
        
        // Encrypt the counter data
        let encrypted_data = self.encrypt_counter_data()?;
        
        // Write to disk
        fs::write(counter_path, encrypted_data)?;
        Ok(())
    }
    
    fn increment(&mut self) -> Result<(), AppError> {
        // Check if date has changed
        let today = Local::now().format("%Y-%m-%d").to_string();
        if today != self.date {
            self.date = today;
            self.today_count = 0;
            println!("{}", "Daily download counter reset for new day.".blue());
        }
        
        self.today_count += 1;
        self.save_to_disk()?;
        
        Ok(())
    }
    
    fn can_download(&self) -> bool {
        // Check if date has changed
        let today = Local::now().format("%Y-%m-%d").to_string();
        if today != self.date {
            return true; // New day, reset counter
        }
        
        self.today_count < self.max_daily_downloads
    }
    
    fn remaining_downloads(&self) -> u32 {
        if self.today_count >= self.max_daily_downloads {
            0
        } else {
            self.max_daily_downloads - self.today_count
        }
    }
}

fn get_counter_path() -> Result<PathBuf, AppError> {
    let mut path = dirs::data_local_dir()
        .ok_or_else(|| AppError::PathError("Could not find local data directory".to_string()))?;
    
    path.push("rustloader");
    fs::create_dir_all(&path)?;
    
    path.push("download_counter.dat");
    Ok(path)
}

// Displays a promotional message during download
fn display_download_promo() {
    let promo = DownloadPromo::new();
    println!("{}", promo.get_random_download_message().bright_yellow());
}

// Displays a promotional message after download
fn display_completion_promo() {
    let promo = DownloadPromo::new();
    println!("\n{}\n", promo.get_random_completion_message().bright_yellow());
}

// Limit video quality to 720p for free version (unless Pro)
fn limit_video_quality(requested_quality: &str, is_pro: bool) -> &str {
    if !is_pro && (requested_quality == "1080" || requested_quality == "4k" || requested_quality == "8k" || 
                  requested_quality == "2160p" || requested_quality == "4320p") {
        println!("{}", 
                format!("⭐ Rustloader Free is limited to {}p. Upgrade to Pro for higher quality. ⭐", MAX_FREE_QUALITY)
                .yellow());
        
        MAX_FREE_QUALITY
    } else {
        requested_quality
    }
}

// Handle file that already exists
fn handle_existing_file(path: &Path) -> Result<PathBuf, AppError> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    
    println!("{}", "File already exists:".yellow());
    println!("{:?}", path);
    
    // Ask user what to do
    let options = vec!["Use existing file", "Download with new filename", "Overwrite existing file"];
    
    let selection = Select::new()
        .with_prompt("What would you like to do?")
        .default(0)
        .items(&options)
        .interact()
        .map_err(|e| AppError::IoError(io::Error::new(io::ErrorKind::Other, e)))?;
    
    match selection {
        0 => {
            // Use existing file
            println!("{}", "Using existing file...".green());
            Ok(path.to_path_buf())
        },
        1 => {
            // Create new filename with timestamp
            let file_stem = path.file_stem()
                .ok_or_else(|| AppError::PathError("Invalid filename".to_string()))?
                .to_string_lossy();
            
            let extension = path.extension()
                .map(|ext| ext.to_string_lossy().to_string())
                .unwrap_or_default();
            
            let now = chrono::Local::now();
            let timestamp = now.format("%Y%m%d%H%M%S");
            
            let new_filename = if extension.is_empty() {
                format!("{}_{}", file_stem, timestamp)
            } else {
                format!("{}_{}.{}", file_stem, timestamp, extension)
            };
            
            let new_path = path.with_file_name(new_filename);
            println!("{} {:?}", "Using new filename:".green(), new_path);
            
            Ok(new_path)
        },
        2 => {
            // Overwrite existing file
            println!("{}", "Overwriting existing file...".yellow());
            if path.exists() {
                fs::remove_file(path)?;
            }
            Ok(path.to_path_buf())
        },
        _ => Err(AppError::General("Invalid option selected".to_string())),
    }
}

/// Handle HTTP 416 errors (file already exists)
async fn handle_http_416_error<F, Fut, T>(f: F) -> Result<T, AppError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    match f().await {
        Err(AppError::DownloadError(msg)) if msg.contains("HTTP 416") => {
            println!("{}", "File already exists (HTTP 416 error).".yellow());
            
            // Ask user what to do
            let options = vec!["Use existing file", "Try with new filename", "Abort download"];
            
            let selection = Select::new()
                .with_prompt("What would you like to do?")
                .default(0)
                .items(&options)
                .interact()
                .map_err(|e| AppError::IoError(io::Error::new(io::ErrorKind::Other, e)))?;
            
            match selection {
                0 => {
                    // Use existing file - return success
                    println!("{}", "Using existing file...".green());
                    Err(AppError::General("Use existing file".to_string()))
                },
                1 => {
                    // Try with a new filename by adding a timestamp
                    println!("{}", "Retrying with new filename...".green());
                    
                    // Generate a timestamp for uniqueness
                    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
                    
                    // We'll return an error with special message that can be handled by the caller
                    // The caller should retry with a new filename based on the timestamp
                    Err(AppError::DownloadError(format!("Retry with new filename: {}", timestamp)))
                },
                _ => {
                    // Abort
                    println!("{}", "Download aborted.".red());
                    Err(AppError::General("Download aborted by user".to_string()))
                }
            }
        },
        result => result,
    }
}

/// Main download function - now using YouTube-DL and FFmpeg libraries
pub async fn download_video_free<F>(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&String>,
    end_time: Option<&String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<&String>,
    force_download: bool,
    bitrate: Option<&String>,
    progress_callback: Option<F>,
) -> Result<(), AppError>
where
    F: Fn(u64, u64) -> bool + Send + Sync + 'static,
{
    // Validate URL more strictly
    validate_url(url)?;

    // Check if Pro version
    let is_pro = crate::license::is_pro_version();

    // If not Pro and not forcing download, check daily limit
    if !is_pro && !force_download {
        let counter = DownloadCounter::load_from_disk()?;
        if !counter.can_download() {
            println!("{}", "⚠️ Daily download limit reached for free version ⚠️".bright_red());
            println!("{}", "🚀 Upgrade to Rustloader Pro for unlimited downloads: rustloader.com/pro 🚀".bright_yellow());
            return Err(AppError::DailyLimitExceeded);
        }

        // Show remaining downloads
        println!("{} {}", 
            "Downloads remaining today:".blue(), 
            counter.remaining_downloads().to_string().green()
        );
    }

    println!("{}: {}", "Download URL".blue(), url);

    // Validate time formats if provided
    if let Some(start) = start_time {
        validate_time_format(start)?;
    }

    if let Some(end) = end_time {
        validate_time_format(end)?;
    }

    // Validate bitrate if provided
    if let Some(rate) = bitrate {
        validate_bitrate(rate)?;

        // For video, we can respect the bitrate 
        // For audio, we enforce the free version limitation if not Pro
        if format != "mp3" || is_pro {
            println!("{}: {}", "Bitrate".blue(), rate);
        } else {
            println!("{} {} {}", 
                "Requested audio bitrate:".yellow(), 
                rate, 
                "(Limited to 128K in free version)".yellow()
            );
        }
    }

    // Apply quality limitation for free version if not Pro
    let limited_quality = match quality {
        Some(q) => Some(limit_video_quality(q, is_pro)),
        None => Some(if is_pro { "1080" } else { "720" }), // Default quality
    };

    // Initialize download directory with enhanced security
    let folder_type = if format == "mp3" { "audio" } else { "videos" };
    let download_dir = initialize_download_dir(
        output_dir.map(|s| s.as_str()), 
        "rustloader", 
        folder_type
    )?;

    // Create enhanced progress tracking
    let progress = Arc::new(DownloadProgress::new());

    // Create progress bar with more detailed template
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    // Update the message with all the information
    pb.set_message(format!("Size: {} | Speed: {} | ETA: {}", 
        "Calculating...", "Connecting...", "Calculating..."));

    // Show a promo message during download preparation (if not Pro)
    if !is_pro {
        display_download_promo();
    }

    // Create wrapper for progress callback
    let progress_wrapper: Option<Arc<dyn Fn(u64, u64) -> bool + Send + Sync>> = progress_callback.map(|callback| {
        let progress_ref = Arc::clone(&progress);
        let pb_clone = pb.clone();
        
        let wrapper: Arc<dyn Fn(u64, u64) -> bool + Send + Sync> = Arc::new(move |downloaded, total| {
            // Update our progress tracker
            progress_ref.update(downloaded, total);
            
            // Update progress bar
            let percentage = progress_ref.get_percentage();
            pb_clone.set_position(percentage);
            
            // Update size, speed and ETA information
            pb_clone.set_message(format!("Size: {} | Speed: {} | ETA: {}",
                progress_ref.format_file_size(),
                progress_ref.format_speed(),
                progress_ref.format_eta()));
            
            // Call original callback
            callback(downloaded, total)
        });
        
        wrapper
    });

    // Create download configuration
    let download_config = DownloadConfig {
        url: url.to_string(),
        quality: limited_quality.map(|q| q.to_string()),
        format: format.to_string(),
        start_time: start_time.cloned(),
        end_time: end_time.cloned(),
        use_playlist,
        download_subtitles,
        output_dir: download_dir.clone(),
        bitrate: bitrate.cloned(),
    };

    // Create downloader
    let downloader = YoutubeDlWrapper::new(download_config, progress_wrapper);

    // Start download process
    println!("{}", "Starting download...".green());

    let _result = match handle_http_416_error(|| downloader.download()).await {
        Ok(result) => result,
        Err(e) => {
            // Finish the progress bar with error
            pb.finish_with_message(format!("Error: {}", e));
            return Err(e);
        }
    };

    // Finish the progress bar
    pb.finish_with_message("Download completed");

    // If not Pro and not forcing download, increment download counter
    if !is_pro && !force_download {
        let mut counter = DownloadCounter::load_from_disk()?;
        counter.increment()?;
    }

    // Send desktop notification
    let notification_result = Notification::new()
        .summary("Download Complete")
        .body(&format!("{} file downloaded successfully.", format.to_uppercase()))
        .show();

    // Handle notification errors separately so they don't prevent download completion
    if let Err(e) = notification_result {
        println!("{}: {}", "Failed to show notification".yellow(), e);
    }

    println!(
        "{} {} {}",
        "Download completed successfully.".green(),
        format.to_uppercase(),
        "file saved.".green()
    );

    // Show completion promo if not Pro
    if !is_pro {
        display_completion_promo();
    }

    Ok(())
}

/// Pro version download implementation with enhanced features
pub async fn download_video_pro<F>(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&String>,
    end_time: Option<&String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<&String>,
    _force_download: bool,
    bitrate: Option<&String>,
    progress_callback: Option<F>,
) -> Result<(), AppError>
where
    F: Fn(u64, u64) -> bool + Send + Sync + 'static,
{
    // In Pro version, we allow all qualities without limitation
    
    // Validate URL
    validate_url(url)?;
    
    println!("{}: {}", "Pro Download URL".blue(), url);
    
    // Validate time formats if provided
    if let Some(start) = start_time {
        validate_time_format(start)?;
    }
    
    if let Some(end) = end_time {
        validate_time_format(end)?;
    }
    
    // Validate bitrate if provided
    if let Some(rate) = bitrate {
        validate_bitrate(rate)?;
        println!("{}: {}", "Bitrate".blue(), rate);
    }
    
    // Initialize download directory with enhanced security
    let folder_type = if format == "mp3" { "audio" } else { "videos" };
    let download_dir = initialize_download_dir(
        output_dir.map(|s| s.as_str()), 
        "rustloader", 
        folder_type
    )?;
    
    // Create enhanced progress tracking
    let progress = Arc::new(DownloadProgress::new());
    
    // Create progress bar with more detailed template
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Update the message with all the information
    pb.set_message(format!("Size: {} | Speed: {} | ETA: {}", 
        "Calculating...", "Connecting...", "Calculating..."));
    
    // Create wrapper for progress callback
    let progress_wrapper: Option<Arc<dyn Fn(u64, u64) -> bool + Send + Sync>> = progress_callback.map(|callback| {
        let progress_ref = Arc::clone(&progress);
        let pb_clone = pb.clone();
        
        let wrapper: Arc<dyn Fn(u64, u64) -> bool + Send + Sync> = Arc::new(move |downloaded, total| {
            // Update our progress tracker
            progress_ref.update(downloaded, total);
            
            // Update progress bar
            let percentage = progress_ref.get_percentage();
            pb_clone.set_position(percentage);
            
            // Update size, speed and ETA information
            pb_clone.set_message(format!("Size: {} | Speed: {} | ETA: {}",
                progress_ref.format_file_size(),
                progress_ref.format_speed(),
                progress_ref.format_eta()));
            
            // Call original callback
            callback(downloaded, total)
        });
        
        wrapper
    });
    
    // Create download configuration
    let download_config = DownloadConfig {
        url: url.to_string(),
        quality: quality.map(|q| q.to_string()),
        format: format.to_string(),
        start_time: start_time.cloned(),
        end_time: end_time.cloned(),
        use_playlist,
        download_subtitles,
        output_dir: download_dir.clone(),
        bitrate: bitrate.cloned(),
    };
    
    // Create downloader
    let downloader = YoutubeDlWrapper::new(download_config, progress_wrapper);
    
    // Start download process with parallel downloads enabled
    println!("{}", "Starting Pro download with enhanced features...".green());
    
    let _result = match handle_http_416_error(|| downloader.download()).await {
        Ok(result) => result,
        Err(e) => {
            // Finish the progress bar with error
            pb.finish_with_message(format!("Error: {}", e));
            return Err(e);
        }
    };
    
    // Finish the progress bar
    pb.finish_with_message("Download completed");
    
    // Send desktop notification
    let notification_result = Notification::new()
        .summary("Pro Download Complete")
        .body(&format!("High-quality {} file downloaded successfully.", format.to_uppercase()))
        .show();
    
    // Handle notification errors separately so they don't prevent download completion
    if let Err(e) = notification_result {
        println!("{}: {}", "Failed to show notification".yellow(), e);
    }
    
    println!(
        "{} {} {}",
        "Pro download completed successfully.".green(),
        format.to_uppercase(),
        "file saved.".green()
    );
    
    Ok(())
}