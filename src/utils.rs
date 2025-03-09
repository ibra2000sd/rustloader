use crate::error::AppError;
use colored::*;
use home::home_dir;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ShellCommand, Stdio};

/// Check if a dependency is installed by searching for it in PATH
pub fn is_dependency_installed(name: &str) -> Result<bool, AppError> {
    let command = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };

    let output = ShellCommand::new(command)
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| AppError::IoError(e))?;

    Ok(output.success())
}

/// Get the version of a dependency
pub fn get_dependency_version(name: &str) -> Result<String, AppError> {
    let output = ShellCommand::new(name)
        .arg("--version")
        .output()
        .map_err(|e| AppError::IoError(e))?;
    
    if !output.status.success() {
        return Err(AppError::General(format!("Failed to get {} version", name)));
    }
    
    let version_output = String::from_utf8_lossy(&output.stdout).to_string();
    // Most programs output version in format "program version x.y.z"
    // Extract just the version number
    let version = version_output
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    
    Ok(version)
}

/// Check if yt-dlp is up to date
pub fn is_ytdlp_updated() -> Result<bool, AppError> {
    let output = ShellCommand::new("yt-dlp")
        .arg("--update")
        .output()
        .map_err(|e| AppError::IoError(e))?;
    
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    // yt-dlp outputs "yt-dlp is up to date" if it's updated
    Ok(output_str.contains("is up to date") || output_str.contains("Updated"))
}

/// Update yt-dlp to latest version
pub fn update_ytdlp() -> Result<(), AppError> {
    println!("{}", "Updating yt-dlp...".blue());
    
    let output = ShellCommand::new("yt-dlp")
        .arg("--update")
        .status()
        .map_err(|e| AppError::IoError(e))?;
    
    if output.success() {
        println!("{}", "yt-dlp updated successfully.".green());
        Ok(())
    } else {
        eprintln!("{}", "Failed to update yt-dlp.".red());
        Err(AppError::General("yt-dlp update failed".to_string()))
    }
}

/// Check if all required dependencies are installed and up to date
pub fn check_dependencies() -> Result<(), AppError> {
    // Check for yt-dlp
    if !is_dependency_installed("yt-dlp")? {
        eprintln!("{}", "yt-dlp is not installed. Please install it and try again.".red());
        return Err(AppError::MissingDependency("yt-dlp".to_string()));
    }

    // Check if yt-dlp is up to date and update if needed
    println!("{}", "Checking if yt-dlp is up to date...".blue());
    match is_ytdlp_updated() {
        Ok(true) => println!("{}", "yt-dlp is up to date.".green()),
        Ok(false) => {
            println!("{}", "yt-dlp needs to be updated.".yellow());
            update_ytdlp()?;
        },
        Err(e) => {
            println!("{}", format!("Could not check yt-dlp version: {}. Continuing anyway.", e).yellow());
        }
    }

    // Check for ffmpeg
    if !is_dependency_installed("ffmpeg")? {
        eprintln!("{}", "ffmpeg is not installed.".yellow());
        return Err(AppError::MissingDependency("ffmpeg".to_string()));
    }

    // For ffmpeg, just print the version since there's no standard way to update it
    match get_dependency_version("ffmpeg") {
        Ok(version) => println!("{} {}", "ffmpeg version:".blue(), version),
        Err(_) => println!("{}", "Could not determine ffmpeg version. Continuing anyway.".yellow())
    }

    Ok(())
}

/// Install ffmpeg based on the current operating system
pub fn install_ffmpeg() -> Result<(), AppError> {
    println!("{}", "Installing ffmpeg...".blue());

    #[cfg(target_os = "macos")]
    {
        let status = ShellCommand::new("brew")
            .arg("install")
            .arg("ffmpeg")
            .status()
            .map_err(|e| AppError::IoError(e))?;

        if status.success() {
            println!("{}", "ffmpeg installed successfully.".green());
        } else {
            eprintln!("{}", "Failed to install ffmpeg. Please install it manually.".red());
            return Err(AppError::General("ffmpeg installation failed.".to_string()));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = ShellCommand::new("sudo")
            .arg("apt")
            .arg("install")
            .arg("-y")
            .arg("ffmpeg")
            .status()
            .map_err(|e| AppError::IoError(e))?;

        if status.success() {
            println!("{}", "ffmpeg installed successfully.".green());
        } else {
            eprintln!("{}", "Failed to install ffmpeg. Please install it manually.".red());
            return Err(AppError::General("ffmpeg installation failed.".to_string()));
        }
    }

    #[cfg(target_os = "windows")]
    {
        println!("{}", "Automatic installation of ffmpeg is not supported on Windows.".yellow());
        println!("{}", "Please download and install ffmpeg manually from: https://ffmpeg.org/download.html".yellow());
        return Err(AppError::General("Automatic ffmpeg installation not supported on Windows.".to_string()));
    }

    Ok(())
}

/// Validate a URL format
pub fn validate_url(url: &str) -> Result<(), AppError> {
    // Basic URL validation - could be improved with more comprehensive checks
    let url_regex = Regex::new(r"^https?://").unwrap();
    
    if !url_regex.is_match(url) {
        return Err(AppError::ValidationError(format!("Invalid URL format: {}", url)));
    }
    
    Ok(())
}

/// Validate time format (HH:MM:SS)
pub fn validate_time_format(time: &str) -> Result<(), AppError> {
    // First check the format using regex
    let re = Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap();
    if !re.is_match(time) {
        return Err(AppError::TimeFormatError(
            "Time must be in the format HH:MM:SS".to_string(),
        ));
    }
    
    // Then check if the values are within valid ranges
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 3 {
        return Err(AppError::TimeFormatError(
            "Time must have hours, minutes, and seconds components".to_string(),
        ));
    }
    
    let hours: u32 = parts[0].parse().map_err(|_| {
        AppError::TimeFormatError("Hours must be a valid number".to_string())
    })?;
    
    let minutes: u32 = parts[1].parse().map_err(|_| {
        AppError::TimeFormatError("Minutes must be a valid number".to_string())
    })?;
    
    let seconds: u32 = parts[2].parse().map_err(|_| {
        AppError::TimeFormatError("Seconds must be a valid number".to_string())
    })?;
    
    if hours >= 24 {
        return Err(AppError::TimeFormatError(
            "Hours must be between 00-23".to_string(),
        ));
    }
    
    if minutes >= 60 {
        return Err(AppError::TimeFormatError(
            "Minutes must be between 00-59".to_string(),
        ));
    }
    
    if seconds >= 60 {
        return Err(AppError::TimeFormatError(
            "Seconds must be between 00-59".to_string(),
        ));
    }
    
    Ok(())
}

/// Initialize the download directory
pub fn initialize_download_dir(
    custom_dir: Option<&str>, 
    program_name: &str, 
    file_type: &str,
) -> Result<PathBuf, AppError> {
    let download_dir = if let Some(dir) = custom_dir {
        PathBuf::from(dir)
    } else {
        match home_dir() {
            Some(mut path) => {
                path.push("Downloads");
                path.push(program_name);
                path.push(file_type);
                path
            }
            None => {
                return Err(AppError::PathError("Failed to find the home directory.".to_string()));
            }
        }
    };

    // Create the directory if it doesn't exist
    if !download_dir.exists() {
        fs::create_dir_all(&download_dir).map_err(|e| {
            eprintln!("{}: {:?}", "Failed to create download directory".red(), e);
            AppError::IoError(e)
        })?;
        println!("{} {:?}", "Created directory:".green(), download_dir);
    }

    Ok(download_dir)
}

/// Format a path for use with yt-dlp
pub fn format_output_path<P: AsRef<Path>>(
    download_dir: P, 
    format: &str
) -> Result<String, AppError> {
    let path_str = download_dir
        .as_ref()
        .join(format!("%(title)s.{}", format))
        .to_str()
        .ok_or_else(|| AppError::PathError("Invalid path encoding".to_string()))?
        .to_string();
    
    Ok(path_str)
}