// src/dependency_validator.rs

use crate::error::AppError;
use std::collections::HashMap;
use std::process::Command;
use colored::*;
use std::env;

/// Information about a dependency
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub is_min_version: bool,
    pub is_vulnerable: bool,
    pub path: Option<String>,
}

/// Check if a command exists
fn command_exists(command: &str) -> bool {
    let cmd = if cfg!(target_os = "windows") {
        Command::new("where").arg(command).output()
    } else {
        Command::new("which").arg(command).output()
    };
    
    cmd.map(|output| output.status.success()).unwrap_or(false)
}

/// Get the path of a command
fn get_command_path(command: &str) -> Option<String> {
    let cmd = if cfg!(target_os = "windows") {
        Command::new("where").arg(command).output()
    } else {
        Command::new("which").arg(command).output()
    };
    
    cmd.ok().and_then(|output| {
        if output.status.success() {
            String::from_utf8(output.stdout)
                .map(|s| s.trim().to_string())
                .ok()
        } else {
            None
        }
    })
}

/// Provides installation instructions based on platform
fn get_installation_instructions(dependency: &str) -> String {
    if cfg!(target_os = "windows") {
        match dependency {
            "yt-dlp" => "Install yt-dlp: pip install yt-dlp".to_string(),
            "ffmpeg" => "Download ffmpeg from https://ffmpeg.org/download.html and add it to your PATH".to_string(),
            _ => format!("Please install {} and make sure it's in your PATH", dependency)
        }
    } else if cfg!(target_os = "macos") {
        match dependency {
            "yt-dlp" => "Install yt-dlp: brew install yt-dlp or pip install yt-dlp".to_string(),
            "ffmpeg" => "Install ffmpeg: brew install ffmpeg".to_string(),
            _ => format!("Please install {} with brew or your package manager", dependency)
        }
    } else {
        // Linux
        match dependency {
            "yt-dlp" => "Install yt-dlp: pip install yt-dlp".to_string(),
            "ffmpeg" => "Install ffmpeg: sudo apt install ffmpeg or your distro's equivalent".to_string(),
            _ => format!("Please install {} with your package manager", dependency)
        }
    }
}

/// Validate that all required dependencies are installed and working
pub fn validate_dependencies() -> Result<HashMap<String, DependencyInfo>, AppError> {
    let mut results = HashMap::new();
    let mut missing_dependencies = Vec::new();
    
    println!("{}", "Checking external dependencies...".blue());
    
    // Check for yt-dlp
    if !command_exists("yt-dlp") {
        println!("{}: {} not found", "ERROR".red(), "yt-dlp");
        println!("{}: {}", "SOLUTION".green(), get_installation_instructions("yt-dlp"));
        missing_dependencies.push("yt-dlp");
    } else {
        let path = get_command_path("yt-dlp");
        let version_output = Command::new("yt-dlp").arg("--version").output().ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
            
        println!("{}: {} (path: {})", "yt-dlp".green(), version_output, 
                path.clone().unwrap_or_else(|| "unknown".to_string()));
                
        results.insert("yt-dlp".to_string(), DependencyInfo {
            name: "yt-dlp".to_string(),
            version: version_output,
            is_min_version: true, // Simplified version check
            is_vulnerable: false,
            path,
        });
    }
    
    // Check for ffmpeg
    if !command_exists("ffmpeg") {
        println!("{}: {} not found", "ERROR".red(), "ffmpeg");
        println!("{}: {}", "SOLUTION".green(), get_installation_instructions("ffmpeg"));
        missing_dependencies.push("ffmpeg");
    } else {
        let path = get_command_path("ffmpeg");
        let version_output = Command::new("ffmpeg").arg("-version").output().ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.lines().next().unwrap_or("unknown").trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
            
        println!("{}: {} (path: {})", "ffmpeg".green(), version_output,
                path.clone().unwrap_or_else(|| "unknown".to_string()));
                
        results.insert("ffmpeg".to_string(), DependencyInfo {
            name: "ffmpeg".to_string(),
            version: version_output,
            is_min_version: true, // Simplified version check
            is_vulnerable: false,
            path,
        });
    }
    
    // Handle missing dependencies
    if !missing_dependencies.is_empty() {
        return Err(AppError::MissingDependency(format!(
            "Missing dependencies: {}. Please install them and try again.", 
            missing_dependencies.join(", ")
        )));
    }
    
    // Check for custom dependency paths in environment variables
    if let Ok(custom_ytdlp_path) = env::var("RUSTLOADER_YTDLP_PATH") {
        println!("{}: Using custom yt-dlp path: {}", "INFO".blue(), custom_ytdlp_path);
        if let Some(info) = results.get_mut("yt-dlp") {
            info.path = Some(custom_ytdlp_path);
        }
    }
    
    if let Ok(custom_ffmpeg_path) = env::var("RUSTLOADER_FFMPEG_PATH") {
        println!("{}: Using custom ffmpeg path: {}", "INFO".blue(), custom_ffmpeg_path);
        if let Some(info) = results.get_mut("ffmpeg") {
            info.path = Some(custom_ffmpeg_path);
        }
    }
    
    println!("{}", "All required dependencies are installed.".green());
    Ok(results)
}