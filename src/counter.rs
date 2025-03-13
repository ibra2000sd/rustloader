// src/counter.rs

use crate::error::AppError;
use chrono::Local;
use dirs_next as dirs;
use std::fs;
use std::path::PathBuf;
use std::fs::File;
use std::io::{Read, Write};

// Constants for free version limitations
const MAX_DAILY_DOWNLOADS: u32 = 5;

/// Check if daily download limit has been reached
pub fn check_daily_limit() -> Result<(), AppError> {
    // Skip limit check if environment variable is set
    if let Ok(bypass_limit) = std::env::var("RUSTLOADER_BYPASS_LIMIT") {
        if bypass_limit == "1" || bypass_limit.to_lowercase() == "true" {
            println!("Notice: Daily limit check bypassed by environment variable");
            return Ok(());
        }
    }
    
    let (count, _) = get_download_counter()?;
    
    if count >= MAX_DAILY_DOWNLOADS {
        return Err(AppError::DailyLimitExceeded);
    }
    
    Ok(())
}

/// Increment daily download count
pub fn increment_daily_count() -> Result<(), AppError> {
    // Skip if environment variable is set
    if let Ok(bypass_limit) = std::env::var("RUSTLOADER_BYPASS_LIMIT") {
        if bypass_limit == "1" || bypass_limit.to_lowercase() == "true" {
            println!("Notice: Download count not incremented due to environment variable");
            return Ok(());
        }
    }
    
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

/// Get current download counter with simplified implementation
fn get_download_counter() -> Result<(u32, String), AppError> {
    // Get counter file path with better error handling
    let counter_path = match get_counter_file_path() {
        Ok(path) => path,
        Err(e) => {
            println!("Warning: Could not determine counter file path: {}", e);
            // Return default values
            return Ok((0, Local::now().format("%Y-%m-%d").to_string()));
        }
    };
    
    // Default values (no downloads today)
    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut count = 0;
    let mut date = today.clone();
    
    // Read counter if exists with better error handling
    if counter_path.exists() {
        match File::open(&counter_path) {
            Ok(mut file) => {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    let parts: Vec<&str> = content.trim().split(',').collect();
                    if parts.len() == 2 {
                        date = parts[0].to_string();
                        count = parts[1].parse().unwrap_or(0);
                        
                        // Reset count if it's a new day
                        if date != today {
                            date = today.clone();
                            count = 0;
                        }
                    }
                }
            },
            Err(e) => {
                println!("Warning: Could not read counter file: {}", e);
                // Continue with default values
            }
        }
    }
    
    Ok((count, date))
}

/// Save download counter with simplified implementation
fn save_download_counter(count: u32, date: &str) -> Result<(), AppError> {
    // Get counter file path
    let counter_path = get_counter_file_path()?;
    
    // Ensure directory exists
    if let Some(parent) = counter_path.parent() {
        if !parent.exists() {
            match fs::create_dir_all(parent) {
                Ok(_) => {},
                Err(e) => {
                    println!("Warning: Could not create counter directory: {}", e);
                    // Even if we can't save, we shouldn't fail the download
                    return Ok(());
                }
            }
        }
    }
    
    // Create content
    let content = format!("{},{}", date, count);
    
    // Write to file with better error handling
    match File::create(&counter_path) {
        Ok(mut file) => {
            match file.write_all(content.as_bytes()) {
                Ok(_) => {},
                Err(e) => {
                    println!("Warning: Could not write to counter file: {}", e);
                    // Even if we can't save, we shouldn't fail the download
                }
            }
        },
        Err(e) => {
            println!("Warning: Could not create counter file: {}", e);
            // Even if we can't save, we shouldn't fail the download
        }
    }
    
    Ok(())
}

/// Get path to counter file with fallbacks
fn get_counter_file_path() -> Result<PathBuf, AppError> {
    // Try to use data_local_dir first
    if let Some(mut data_dir) = dirs::data_local_dir() {
        data_dir.push("rustloader");
        // Try to create the directory
        let _ = fs::create_dir_all(&data_dir);
        data_dir.push("counter.dat");
        return Ok(data_dir);
    }
    
    // Fall back to home directory
    if let Some(mut home_dir) = dirs::home_dir() {
        home_dir.push(".rustloader");
        // Try to create the directory
        let _ = fs::create_dir_all(&home_dir);
        home_dir.push("counter.dat");
        return Ok(home_dir);
    }
    
    // Last resort: temp directory
    let mut temp_dir = std::env::temp_dir();
    temp_dir.push("rustloader");
    // Try to create the directory
    let _ = fs::create_dir_all(&temp_dir);
    temp_dir.push("counter.dat");
    Ok(temp_dir)
}