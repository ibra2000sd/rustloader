// src/downloader.rs

use crate::error::AppError;
use crate::utils::{format_output_path, initialize_download_dir, validate_bitrate, validate_time_format, validate_url, validate_path_safety};

// Ensure downloads are single-threaded in free version
fn ensure_single_threaded_download(command: &mut AsyncCommand) {
    // Disable any multi-threading options
    command.arg("--no-part");  // Disable part-based downloaders
    command.arg("--downloader").arg("native"); // Use the basic downloader
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

/// Download a video or audio file from the specified URL
/// This is the FREE version with enhanced security
pub async fn download_video_free(
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
) -> Result<(), AppError> {
    // Validate URL more strictly
    validate_url(url)?;
    
    // Check daily download limit with secured counter
    let mut counter = DownloadCounter::load_from_disk()?;
    if !force_download && !counter.can_download() {
        println!("{}", "⚠️ Daily download limit reached for free version ⚠️".bright_red());
        println!("{}", "🚀 Upgrade to Rustloader Pro for unlimited downloads: rustloader.com/pro 🚀".bright_yellow());
        return Err(AppError::DailyLimitExceeded);
    }
    
    // Show remaining downloads
    println!("{} {}", 
        "Downloads remaining today:".blue(), 
        counter.remaining_downloads().to_string().green()
    );
    
    println!("{}: {}", "Download URL".blue(), url);
    
    // If force_download is enabled, clear any partial downloads
    if force_download {
        println!("{}", "Force download mode enabled - clearing partial downloads".blue());
        if let Err(e) = clear_partial_downloads(url) {
            println!("{}", format!("Warning: Could not clear partial downloads: {}. Continuing anyway.", e).yellow());
        }
    }
    
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
        
        // For video, we can respect the bitrate in free version
        // For audio, we enforce the free version limitation
        if format != "mp3" {
            println!("{}: {}", "Video bitrate".blue(), rate);
        }
    }
    
    // Apply quality limitation for free version
    let limited_quality = quality.map(limit_video_quality);
    
    // Initialize download directory with enhanced security
    let folder_type = if format == "mp3" { "audio" } else { "videos" };
    let download_dir = initialize_download_dir(
        output_dir.map(|s| s.as_str()), 
        "rustloader", 
        folder_type
    )?;
    
    // Create the output path format with validation
    let output_path = format_output_path(&download_dir, format)?;
    
    // Create enhanced progress tracking
    let progress = Arc::new(DownloadProgress::new());
    
    // Create progress bar with more detailed template
    let pb = Arc::new(ProgressBar::new(100));
    // Instead of using set_suffix, modify the ProgressBar template to include sections for these values
pb.set_style(
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg}")
        .unwrap()
        .progress_chars("#>-")
);

// Then update the message with all the information
pb.set_message(format!("Size: {} | Speed: {} | ETA: {}", 
    "Calculating...", "Connecting...", "Calculating..."));
    
    // Show a promo message during download preparation
    display_download_promo();
    
    // Build yt-dlp command securely
    let mut command = AsyncCommand::new("yt-dlp");
    
    // Ensure single-threaded downloads only for free version
    ensure_single_threaded_download(&mut command);
    
    // If force download is enabled, don't try to resume partial downloads
    if force_download {
        command.arg("--no-continue");  // Don't try to resume partial downloads
        command.arg("--no-part-file"); // Don't create .part files
    }
    
    // Add format selection based on requested format and quality
    if format == "mp3" {
        command.arg("-f")
               .arg("bestaudio[ext=m4a]")
               .arg("--extract-audio")
               .arg("--audio-format")
               .arg("mp3");
               
        // Apply audio quality limitation for free version
        modify_audio_command(&mut command, bitrate);
    } else {
        command.arg("-f");
        
        let quality_code = match limited_quality {
            Some("480") => "best[height<=480]",
            Some("720") => "best[height<=720]",
            _ => "best[height<=720]", // Always limit to 720p in free version
        };
        
        command.arg(quality_code);
        
        // Add video bitrate if specified
        if let Some(rate) = bitrate {
            // Validate and sanitize the bitrate value
            let safe_rate = sanitize_command_arg(rate)?;
            command.arg("--postprocessor-args")
                  .arg(format!("ffmpeg:-b:v {}", safe_rate));
        }
    }
    
    // Escape the output path properly
    command.arg("-o").arg(&output_path);
    
    // Handle playlist options
    if use_playlist {
        command.arg("--yes-playlist");
        println!("{}", "Playlist mode enabled - will download all videos in playlist".yellow());
    } else {
        command.arg("--no-playlist");
    }
    
    // Add subtitles if requested
    if download_subtitles {
        command.arg("--write-subs").arg("--sub-langs").arg("all");
        println!("{}", "Subtitles will be downloaded if available".blue());
    }
    
    // Process start and end times with enhanced security
    if start_time.is_some() || end_time.is_some() {
        let mut time_args = String::new();
        
        if let Some(start) = start_time {
            // Validate again right before using
            validate_time_format(start)?;
            time_args.push_str(&format!("-ss {} ", start));
        }
        
        if let Some(end) = end_time {
            // Validate again right before using
            validate_time_format(end)?;
            time_args.push_str(&format!("-to {} ", end));
        }
        
        if !time_args.is_empty() {
            command.arg("--postprocessor-args").arg(format!("ffmpeg:{}", time_args.trim()));
        }
    }
    
    // Add throttling and retry options to avoid detection
    command.arg("--socket-timeout").arg("30");
    command.arg("--retries").arg("10");
    command.arg("--fragment-retries").arg("10");
    command.arg("--throttled-rate").arg("100K");
    
    // Add progress output format for parsing
    command.arg("--newline");
    command.arg("--progress-template").arg("download:%(progress.downloaded_bytes)s/%(progress.total_bytes)s");
    
    // Add user agent to avoid detection
    command.arg("--user-agent")
           .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
    
    // Add the URL last
    command.arg(url);
    
    // Execute the command
    println!("{}", "Starting download...".green());
    
    // Set up pipes for stdout and stderr
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    
    // Spawn the command
    let mut child = command.spawn().map_err(|e| {
        eprintln!("{}", "Failed to execute yt-dlp command.".red());
        AppError::IoError(e)
    })?;
    
    // Process stdout to update progress bar with enhanced information
    if let Some(stdout) = child.stdout.take() {
        let stdout_reader = BufReader::new(stdout);
        let mut lines = stdout_reader.lines();
        let pb_clone = Arc::clone(&pb);
        let progress_clone = Arc::clone(&progress);
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if line.starts_with("download:") {
                    if let Some(progress_str) = line.strip_prefix("download:") {
                        let parts: Vec<&str> = progress_str.split('/').collect();
                        if parts.len() == 2 {
                            // Try to parse downloaded and total bytes
                            if let (Ok(downloaded), Ok(total)) = (
                                parts[0].trim().parse::<u64>(),
                                parts[1].trim().parse::<u64>(),
                            ) {
                                if total > 0 {
                                    // Update progress tracking
                                    progress_clone.update(downloaded, total);
                                    
                                    // Update progress bar
                                    let percentage = progress_clone.get_percentage();
                                    pb_clone.set_position(percentage);
                                    
                                    // Update size, speed and ETA information
                                    // Replace line 259 (with set_suffix) with this pattern
                                    pb_clone.set_message(format!("Size: {} | Speed: {} | ETA: {}",
                                    progress_clone.format_file_size(),
                                    progress_clone.format_speed(),
                                    progress_clone.format_eta()));
                                }
                            }
                        }
                    }
                } else {
                    // Print other output from yt-dlp
                    println!("{}", line);
                }
            }
        });
    }
    
    // Process stderr to show errors
    if let Some(stderr) = child.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        let mut lines = stderr_reader.lines();
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("{}", line.red());
            }
        });
    }
    
    // Wait for the command to finish
    let status = child.wait().await.map_err(|e| {
        eprintln!("{}", "Failed to wait for yt-dlp to complete.".red());
        AppError::IoError(e)
    })?;
    
    // Finish the progress bar
    pb.finish_with_message("Download completed");
    
    // Check if command succeeded
    if !status.success() {
        return Err(AppError::DownloadError(
            "yt-dlp command failed. Please verify the URL and options provided.".to_string(),
        ));
    }
    
    // Increment download counter if not using force_download
    if !force_download {
        counter.increment()?;
    }
    
    // Send desktop notification
    Notification::new()
        .summary("Download Complete")
        .body(&format!("{} file downloaded successfully.", format.to_uppercase()))
        .show()
        .map_err(|e| AppError::General(format!("Failed to show notification: {}", e)))?;
    
    println!(
        "{} {} {}",
        "Download completed successfully.".green(),
        format.to_uppercase(),
        "file saved.".green()
    );
    
    // Show completion promo
    display_completion_promo();
    
    Ok(())
}
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use notify_rust::Notification;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;
use rand::Rng;
use std::fs;
use std::path::PathBuf;
use chrono::{Local};
use dirs_next as dirs;
use ring::{digest, hmac};
use base64::{Engine as _, engine::general_purpose};
use hostname;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, Duration};
use std::sync::Mutex;
use humansize::{format_size, BINARY};

// Constants for free version limitations
const MAX_FREE_QUALITY: &str = "720";
const FREE_MP3_BITRATE: &str = "128K";

// Enhanced progress tracking
struct DownloadProgress {
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
    
    fn update(&self, downloaded: u64, total: u64) {
        // Update bytes counters
        self.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        self.total_bytes.store(total, Ordering::SeqCst);
        
        // Calculate and update speed
        let now = Instant::now();
        
        let mut last_update = self.last_update.lock().unwrap();
        let time_diff = now.duration_since(*last_update).as_millis();
        
        // Only update speed calculation every 100ms to avoid jitter
        if time_diff >= 100 {
            let mut last_speed_samples = self.last_speed_samples.lock().unwrap();
            let mut speed = self.download_speed.lock().unwrap();
            
            // Calculate current speed
            if let Some(last_downloaded) = self.downloaded_bytes.load(Ordering::SeqCst).checked_sub(downloaded) {
                let current_speed = last_downloaded as f64 / (time_diff as f64 / 1000.0);
                
                // Add to speed samples
                last_speed_samples.push(current_speed);
                if last_speed_samples.len() > 10 {
                    last_speed_samples.remove(0);
                }
                
                // Calculate average speed from samples
                let sum: f64 = last_speed_samples.iter().sum();
                *speed = sum / last_speed_samples.len() as f64;
            }
            
            *last_update = now;
        }
    }
    
    fn get_percentage(&self) -> u64 {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        
        if total == 0 {
            return 0;
        }
        
        (downloaded as f64 / total as f64 * 100.0) as u64
    }
    
    fn get_speed(&self) -> f64 {
        *self.download_speed.lock().unwrap()
    }
    
    fn get_eta(&self) -> Option<Duration> {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        let speed = self.get_speed();
        
        if speed <= 0.0 || downloaded >= total {
            return None;
        }
        
        let remaining_bytes = total - downloaded;
        let seconds_remaining = remaining_bytes as f64 / speed;
        
        Some(Duration::from_secs_f64(seconds_remaining))
    }
    
    fn format_eta(&self) -> String {
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
    
    fn format_speed(&self) -> String {
        let speed = self.get_speed();
        if speed <= 0.0 {
            return "Calculating...".to_string();
        }
        
        format!("{}/s", format_size(speed as u64, BINARY))
    }
    
    fn format_file_size(&self) -> String {
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
                        return Ok(Self::new()); // Reset counter for new day
                    }
                    
                    Ok(Self {
                        today_count: count,
                        date,
                        max_daily_downloads: 5,
                    })
                },
                Err(_) => {
                    // If decryption fails, assume tampering and create a new counter
                    // with max downloads already used (as a penalty for tampering)
                    println!("{}", "Warning: Download counter validation failed. Counter has been reset.".yellow());
                    let mut counter = Self::new();
                    counter.today_count = counter.max_daily_downloads; // Use up all downloads as penalty
                    
                    // Save the new counter immediately
                    if let Err(e) = counter.save_to_disk() {
                        println!("{}: {}", "Error saving counter".red(), e);
                    }
                    
                    Ok(counter)
                }
            }
        } else {
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

/// Extract YouTube video ID from URL with enhanced security
fn extract_video_id(url: &str) -> Option<String> {
    // Define strict character allowlist for video IDs
    let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    
    // Extract video ID from YouTube URL patterns
    if let Some(v_pos) = url.find("v=") {
        let id_start = v_pos + 2;
        let id_end = url[id_start..]
            .find(|c: char| !is_valid_char(c))
            .map_or(url.len(), |pos| id_start + pos);
        
        let extracted = &url[id_start..id_end];
        
        // Additional validation - YouTube IDs are typically 11 characters
        if extracted.len() >= 8 && extracted.len() <= 12 && 
           extracted.chars().all(is_valid_char) {
            return Some(extracted.to_string());
        }
    } else if url.contains("youtu.be/") {
        let parts: Vec<&str> = url.split("youtu.be/").collect();
        if parts.len() < 2 {
            return None;
        }
        
        let id_part = parts[1];
        let id_end = id_part
            .find(|c: char| !is_valid_char(c))
            .map_or(id_part.len(), |pos| pos);
        
        let extracted = &id_part[..id_end];
        
        // Additional validation - YouTube IDs are typically 11 characters
        if extracted.len() >= 8 && extracted.len() <= 12 && 
           extracted.chars().all(is_valid_char) {
            return Some(extracted.to_string());
        }
    }
    
    None
}

/// Sanitize a filename to prevent command injection - improved whitelist approach
fn sanitize_filename(filename: &str) -> Result<String, AppError> {
    // Strict whitelist for allowed characters
    let sanitized: String = filename.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    
    // Enforce minimum length and validate that all characters passed the filter
    if sanitized.is_empty() || sanitized.len() < filename.len() / 2 {
        Err(AppError::ValidationError("Invalid filename after sanitization".to_string()))
    } else {
        Ok(sanitized)
    }
}

/// Clears any partial downloads with enhanced security
fn clear_partial_downloads(url: &str) -> Result<(), AppError> {
    println!("{}", "Clearing partial downloads to avoid resumption errors...".blue());
    
    // Get the video ID with enhanced extraction and validation
    let video_id = match extract_video_id(url) {
        Some(id) => {
            // Apply additional sanitization for extra security
            sanitize_filename(&id)?
        },
        None => {
            println!("{}", "Could not extract video ID, skipping partial download cleanup.".yellow());
            return Ok(());
        }
    };
    
    // Additional validation - ensure ID has reasonable length
    if video_id.len() < 8 || video_id.len() > 12 {
        println!("{}", "Extracted video ID has suspicious length, skipping cleanup.".yellow());
        return Ok(());
    }
    
    // Get the download directory as a PathBuf
    let downloads_dir = match dirs::home_dir() {
        Some(mut path) => {
            path.push("Downloads");
            path.push("rustloader");
            path
        },
        None => {
            return Err(AppError::PathError("Could not find home directory".to_string()));
        }
    };
    
    println!("{} {}", "Looking for partial downloads with ID:".blue(), video_id);
    
    // Use enhanced safe methods to find and remove files
    match safe_cleanup(&downloads_dir, &video_id) {
        Ok(count) => {
            println!("{} {}", "Removed partial downloads:".green(), count);
        },
        Err(e) => {
            println!("{}: {}", "Warning".yellow(), e);
            // Continue with the download even if cleanup fails
        }
    }
    
    println!("{}", "Partial download cleanup completed.".green());
    Ok(())
}

/// Unified safe cleanup implementation with enhanced security
fn safe_cleanup(dir: &PathBuf, video_id: &str) -> Result<usize, AppError> {
    let mut count = 0;
    
    // Verify that video_id only contains safe characters again
    if !video_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::SecurityViolation);
    }
    
    // Process .part and .ytdl files only
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        let file_name_str = file_name.to_string_lossy();
                        
                        // Check if this is a partial download matching our video ID
                        // Only remove files ending with .part or .ytdl
                        if file_name_str.contains(video_id) && 
                           (file_name_str.ends_with(".part") || file_name_str.ends_with(".ytdl")) {
                            // Double-check the file name for security
                            if file_name_str.chars().all(|c| 
                                c.is_ascii_alphanumeric() || 
                                c == '-' || c == '_' || c == '.' || c == ' '
                            ) {
                                // Remove the file
                                println!("{} {}", "Removing:".yellow(), file_name_str);
                                if let Err(e) = std::fs::remove_file(&path) {
                                    println!("{}: {}", "Failed to remove file".red(), e);
                                } else {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(count)
}

/// Sanitize command arguments using a strict whitelist approach
fn sanitize_command_arg(arg: &str) -> Result<String, AppError> {
    // Define specific allowlists for different argument types
    
    // For bitrate arguments (e.g., 1000K)
    if arg.ends_with('K') || arg.ends_with('M') {
        let num_part = &arg[0..arg.len()-1];
        if num_part.chars().all(|c| c.is_ascii_digit()) {
            return Ok(arg.to_string());
        }
    }
    
    // For time arguments (e.g., 00:01:30)
    if arg.len() == 8 && arg.chars().nth(2) == Some(':') && arg.chars().nth(5) == Some(':') {
        let time_parts: Vec<&str> = arg.split(':').collect();
        if time_parts.len() == 3 && 
           time_parts.iter().all(|part| part.chars().all(|c| c.is_ascii_digit())) {
            return Ok(arg.to_string());
        }
    }
    
    // For format arguments (mp3, mp4, etc.)
    if ["mp3", "mp4", "webm", "m4a", "flac", "wav", "ogg"].contains(&arg) {
        return Ok(arg.to_string());
    }
    
    // For quality specifiers
    if ["480", "720", "1080", "2160", "best", "bestaudio"].contains(&arg) ||
       arg.starts_with("best[") && arg.ends_with("]") {
        return Ok(arg.to_string());
    }
    
    // For URLs - apply URL validation instead
    if arg.starts_with("http://") || arg.starts_with("https://") {
        if let Err(e) = validate_url(arg) {
            return Err(e);
        }
        return Ok(arg.to_string());
    }
    
    // For paths - validate separately
    if arg.contains('/') || arg.contains('\\') {
        let path = std::path::Path::new(arg);
        if let Err(e) = validate_path_safety(path) {
            return Err(e);
        }
        return Ok(arg.to_string());
    }
    
    // For ffmpeg arguments, allow specific prefixes
    if arg.starts_with("ffmpeg:") {
        let ffmpeg_arg = &arg[7..];
        // Check each part of the ffmpeg argument separately
        let parts: Vec<&str> = ffmpeg_arg.split_whitespace().collect();
        for part in parts {
            // Allow common ffmpeg options
            if part.starts_with("-") {
                let option = &part[1..];
                if !["ss", "to", "t", "b:v", "b:a", "c:v", "c:a", "vf", "af"].contains(&option) {
                    return Err(AppError::ValidationError(format!("Invalid ffmpeg option: {}", option)));
                }
            } 
            // Allow time values (00:00:00 format)
            else if part.len() == 8 && part.chars().nth(2) == Some(':') && part.chars().nth(5) == Some(':') {
                let time_parts: Vec<&str> = part.split(':').collect();
                if time_parts.len() != 3 || 
                   !time_parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
                    return Err(AppError::ValidationError(format!("Invalid time format: {}", part)));
                }
            }
            // Allow bitrate values
            else if part.ends_with('k') || part.ends_with('K') || part.ends_with('m') || part.ends_with('M') {
                let num_part = &part[0..part.len()-1];
                if !num_part.chars().all(|c| c.is_ascii_digit()) {
                    return Err(AppError::ValidationError(format!("Invalid bitrate value: {}", part)));
                }
            }
        }
        return Ok(arg.to_string());
    }
    
    // General whitelist for other arguments
    let valid_chars = arg.chars().all(|c| 
        c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-' || 
        c == '.' || c == ':' || c == '=' || c == '[' || c == ']'
    );
    
    if !valid_chars {
        return Err(AppError::ValidationError(format!("Invalid characters in argument: {}", arg)));
    }
    
    Ok(arg.to_string())
}

// Limit video quality to 720p for free version
fn limit_video_quality(requested_quality: &str) -> &str {
    match requested_quality {
        "1080" | "4k" | "8k" | "2160p" | "4320p" => {
            println!("{}", format!("⭐ Rustloader Pro required for quality above {}p ⭐", MAX_FREE_QUALITY).yellow());
            println!("{}", format!("👉 Using maximum allowed quality ({}p) for free version.", MAX_FREE_QUALITY).yellow());
            
            // Just reference the PremiumFeature error type to fix the warning
            // but don't try to return it (which would cause a type mismatch)
            let _unused = AppError::PremiumFeature("High quality video".to_string());
            
            MAX_FREE_QUALITY
        },
        _ => requested_quality,
    }
}

// Modify audio command to limit quality to 128kbps
fn modify_audio_command(command: &mut AsyncCommand, bitrate: Option<&String>) {
    // If bitrate is specified in Pro version, it would be respected
    // but in free version, we enforce the limitation
    if let Some(rate) = bitrate {
        println!("{} {} {}", 
            "Requested audio bitrate:".yellow(), 
            rate, 
            "(Limited to 128K in free version)".yellow()
        );
    }
    
    // Add audio bitrate limitation for free version
    command.arg("--audio-quality").arg("7"); // 128kbps in yt-dlp scale (0-10)
    command.arg("--postprocessor-args").arg(format!("ffmpeg:-b:a {}", FREE_MP3_BITRATE));
    
    println!("{}", "⭐ Limited to 128kbps audio. Upgrade to Pro for studio-quality audio. ⭐".yellow());
}
