// src/utils.rs (partial, focused on path handling)

use crate::error::AppError;
use home::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// Initialize download directory with improved error handling
pub fn initialize_download_dir(
    custom_dir: Option<&str>, 
    program_name: &str, 
    file_type: &str,
) -> Result<PathBuf, AppError> {
    let download_dir = if let Some(dir) = custom_dir {
        // User specified a custom directory
        let path = PathBuf::from(dir);
        
        // More lenient path validation - just check that it exists or can be created
        if !path.exists() {
            println!("Custom directory doesn't exist, will attempt to create it: {:?}", path);
            fs::create_dir_all(&path).map_err(|e| {
                eprintln!("Failed to create custom directory: {:?}", e);
                AppError::PathError(format!("Failed to create directory '{}': {}", dir, e))
            })?;
            println!("Created custom directory: {:?}", path);
        }
        
        path
    } else {
        // Use default path under home directory with better fallbacks
        match home_dir() {
            Some(mut path) {
                // Start with ~/Downloads if it exists
                path.push("Downloads");
                if !path.exists() {
                    // Fall back to home directory if Downloads doesn't exist
                    path = home_dir().unwrap();
                }
                
                // Add program subdirectories
                path.push(program_name);
                path.push(file_type);
                
                path
            }
            None => {
                // If home dir detection fails, use current directory
                println!("Could not detect home directory, using current directory");
                let mut path = std::env::current_dir().map_err(|e| {
                    AppError::PathError(format!("Failed to get current directory: {}", e))
                })?;
                path.push(program_name);
                path.push(file_type);
                path
            }
        }
    };

    // Create the directory if it doesn't exist
    if !download_dir.exists() {
        fs::create_dir_all(&download_dir).map_err(|e| {
            eprintln!("Failed to create download directory: {:?}", e);
            AppError::IoError(e)
        })?;
        println!("Created directory: {:?}", download_dir);
    }

    println!("Using download directory: {:?}", download_dir);
    Ok(download_dir)
}

/// Validate URL format with more permissive approach
pub fn validate_url(url: &str) -> Result<(), AppError> {
    // Basic checks to ensure it's a valid URL
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::ValidationError(format!("Invalid URL format: {}. URL must start with http:// or https://", url)));
    }
    
    // Allow any YouTube, Vimeo, Dailymotion, etc. URL formats
    if url.contains("youtube.com") || url.contains("youtu.be") ||
       url.contains("vimeo.com") || url.contains("dailymotion.com") ||
       url.contains("facebook.com") || url.contains("twitter.com") {
        return Ok(());
    }
    
    // For other URLs, do a basic format check
    if url.contains(".") && !url.contains("localhost") && !url.contains("127.0.0.1") {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!("Invalid URL or local URLs not supported: {}", url)))
    }
}

/// Format a safe path for use with yt-dlp - more permissive version
pub fn format_output_path<P: AsRef<Path>>(
    download_dir: P, 
    format: &str
) -> Result<String, AppError> {
    // Simplified format validation
    match format {
        "mp3" | "mp4" | "webm" | "m4a" | "flac" | "wav" | "ogg" => {},
        _ => return Err(AppError::ValidationError(format!("Invalid output format: {}", format))),
    }
    
    // Use PathBuf for proper platform-specific path handling
    let path_buf = download_dir.as_ref().join(format!("%(title)s.{}", format));
    
    let path_str = path_buf
        .to_str()
        .ok_or_else(|| AppError::PathError("Invalid path encoding".to_string()))?
        .to_string();
    
    Ok(path_str)
}