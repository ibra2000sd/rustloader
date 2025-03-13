// src/counter.rs
use crate::error::AppError;
use chrono::Local;
use dirs_next as dirs;
use std::fs;
use ring::hmac;
use std::path::PathBuf;
use base64::{Engine as _, engine::general_purpose};

// Constants for free version limitations
const MAX_DAILY_DOWNLOADS: u32 = 5;

/// Check if daily download limit has been reached
pub fn check_daily_limit() -> Result<(), AppError> {
    let (count, _) = get_download_counter()?;
    
    if count >= MAX_DAILY_DOWNLOADS {
        return Err(AppError::DailyLimitExceeded);
    }
    
    Ok(())
}

/// Increment daily download count
pub fn increment_daily_count() -> Result<(), AppError> {
    let (count, date) = get_download_counter()?;
    
    // Check if we're still counting for today
    let today = Local::now().format("%Y-%m-%d").to_string();
    let new_count = if date == today {
        count + 1
    } else {
        1 // First download of a new day
    };
    
    save_download_counter(new_count, &today)?;
    
    Ok(())
}

/// Get current download counter
fn get_download_counter() -> Result<(u32, String), AppError> {
    // Get counter file path
    let counter_path = get_counter_file_path()?;
    
    // Default values (no downloads today)
    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut count = 0;
    let mut date = today.clone();
    
    // Read counter if exists
    if counter_path.exists() {
        match fs::read_to_string(&counter_path) {
            Ok(content) => {
                // Decrypt and verify content
                if let Ok((stored_date, stored_count)) = decrypt_counter(&content) {
                    // If date matches today, use the stored count
                    // otherwise start fresh for a new day
                    if stored_date == today {
                        count = stored_count;
                        date = stored_date;
                    }
                }
            },
            Err(_) => {
                // Counter file exists but couldn't be read - start fresh
            }
        }
    }
    
    Ok((count, date))
}

/// Save download counter
fn save_download_counter(count: u32, date: &str) -> Result<(), AppError> {
    // Get counter file path
    let counter_path = get_counter_file_path()?;
    
    // Ensure directory exists
    if let Some(parent) = counter_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Encrypt counter data
    let encrypted = encrypt_counter(date, count)?;
    
    // Write to file
    fs::write(counter_path, encrypted)?;
    
    Ok(())
}

/// Get path to counter file
fn get_counter_file_path() -> Result<PathBuf, AppError> {
    let mut data_dir = dirs::data_local_dir()
        .ok_or_else(|| AppError::PathError("Could not locate local data directory".to_string()))?;
    
    data_dir.push("rustloader");
    data_dir.push("counter.dat");
    
    Ok(data_dir)
}

/// Encrypt counter data
fn encrypt_counter(date: &str, count: u32) -> Result<String, AppError> {
    // Create the data string
    let content = format!("{},{}", date, count);
    
    // Create HMAC signature with a simple key (in a real app, use a better key)
    let key = hmac::Key::new(hmac::HMAC_SHA256, b"rustloader-counter-key");
    let signature = hmac::sign(&key, content.as_bytes());
    let signature_b64 = general_purpose::STANDARD.encode(signature.as_ref());
    
    // Combine data and signature
    let full_data = format!("{}\n{}", content, signature_b64);
    
    // Base64 encode the full data
    Ok(general_purpose::STANDARD.encode(full_data.as_bytes()))
}

/// Decrypt counter data
fn decrypt_counter(encrypted_data: &str) -> Result<(String, u32), AppError> {
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
    let key = hmac::Key::new(hmac::HMAC_SHA256, b"rustloader-counter-key");
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