// src/downloader.rs
//
// Fixed version with corrected syntax errors

use crate::error::AppError;
use crate::youtube_dl_wrapper::{YoutubeDlWrapper, DownloadConfig, ProgressCallback};
use crate::utils::{initialize_download_dir, validate_bitrate, validate_time_format, validate_url};
use crate::ffmpeg_wrapper;
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
#[derive(Debug)]
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

// ... rest of your implementation here ...

// Make sure the file ends with a proper closing brace for the last module/impl block

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
}