// src/dependency_validator.rs

use crate::error::AppError;
use std::collections::HashMap;
use std::process::Command;
use colored::*;
use std::ffi::OsStr;

/// Information about a dependency
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub is_min_version: bool,
    pub is_vulnerable: bool,
    pub path: Option<String>,
}

/// Validate all required dependencies
pub fn validate_dependencies() -> Result<HashMap<String, DependencyInfo>, AppError> {
    let mut results = HashMap::new();
    let mut has_issues = false;
    
    println!("{}", "Validating external dependencies...".blue());
    
    // Check for yt-dlp
    match validate_ytdlp() {
        Ok(info) => {
            if !info.is_min_version {
                has_issues = true;
                println!("{}: yt-dlp version {} is below minimum required", 
                    "WARNING".yellow(),
                    info.version);
            } else {
                println!("{}: {} ({})", "yt-dlp".green(), info.version, 
                    info.path.as_deref().unwrap_or("unknown path"));
            }
            results.insert("yt-dlp".to_string(), info);
        },
        Err(e) => {
            has_issues = true;
            println!("{}: {}", "ERROR".red(), e);
            results.insert("yt-dlp".to_string(), DependencyInfo {
                name: "yt-dlp".to_string(),
                version: "not found".to_string(),
                is_min_version: false,
                is_vulnerable: false,
                path: None,
            });
        }
    }
    
    // Check for ffmpeg
    match validate_ffmpeg() {
        Ok(info) => {
            if !info.is_min_version {
                has_issues = true;
                println!("{}: ffmpeg version {} is below minimum required", 
                    "WARNING".yellow(),
                    info.version);
            } else {
                println!("{}: {} ({})", "ffmpeg".green(), info.version, 
                    info.path.as_deref().unwrap_or("unknown path"));
            }
            results.insert("ffmpeg".to_string(), info);
        },
        Err(e) => {
            has_issues = true;
            println!("{}: {}", "ERROR".red(), e);
            results.insert("ffmpeg".to_string(), DependencyInfo {
                name: "ffmpeg".to_string(),
                version: "not found".to_string(),
                is_min_version: false,
                is_vulnerable: false,
                path: None,
            });
        }
    }
    
    // Display summary
    if has_issues {
        println!("{}", "\nDependency validation completed with warnings.".yellow());
    } else {
        println!("{}", "\nAll dependencies validated successfully.".green());
    }
    
    Ok(results)
}

/// Validate yt-dlp installation
fn validate_ytdlp() -> Result<DependencyInfo, AppError> {
    // First check if yt-dlp is in PATH
    let path = find_executable("yt-dlp")?;
    
    // Get version
    let output = Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|e| AppError::MissingDependency(format!("Failed to run yt-dlp: {}", e)))?;
    
    if !output.status.success() {
        return Err(AppError::MissingDependency("yt-dlp returned error status".to_string()));
    }
    
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Check for minimum version (2022.01.21 or later)
    let is_min_version = is_ytdlp_version_sufficient(&version);
    
    // Check for known vulnerabilities (can be expanded)
    let is_vulnerable = false;
    
    Ok(DependencyInfo {
        name: "yt-dlp".to_string(),
        version,
        is_min_version,
        is_vulnerable,
        path: Some(path),
    })
}

/// Validate ffmpeg installation
fn validate_ffmpeg() -> Result<DependencyInfo, AppError> {
    // First check if ffmpeg is in PATH
    let path = find_executable("ffmpeg")?;
    
    // Get version
    let output = Command::new(&path)
        .arg("-version")
        .output()
        .map_err(|e| AppError::MissingDependency(format!("Failed to run ffmpeg: {}", e)))?;
    
    if !output.status.success() {
        return Err(AppError::MissingDependency("ffmpeg returned error status".to_string()));
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Extract version from output
    let version = match output_str.lines().next() {
        Some(line) => {
            if let Some(version_start) = line.find("version ") {
                let version_str = &line[version_start + 8..];
                if let Some(version_end) = version_str.find(' ') {
                    version_str[..version_end].to_string()
                } else {
                    version_str.to_string()
                }
            } else {
                "unknown".to_string()
            }
        },
        None => "unknown".to_string(),
    };
    
    // Check for minimum version (4.0 or later)
    let is_min_version = is_ffmpeg_version_sufficient(&version);
    
    // Check for known vulnerabilities (can be expanded)
    let is_vulnerable = false;
    
    Ok(DependencyInfo {
        name: "ffmpeg".to_string(),
        version,
        is_min_version,
        is_vulnerable,
        path: Some(path),
    })
}

/// Find an executable in PATH
fn find_executable<S: AsRef<OsStr>>(name: S) -> Result<String, AppError> {
    let which_command = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    
    let output = Command::new(which_command)
        .arg(name.as_ref())
        .output()
        .map_err(|e| AppError::MissingDependency(format!("Failed to locate executable: {}", e)))?;
    
    if !output.status.success() {
        return Err(AppError::MissingDependency(format!("Executable '{}' not found in PATH", 
            name.as_ref().to_string_lossy())));
    }
    
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(path)
}

/// Check if yt-dlp version is sufficient
fn is_ytdlp_version_sufficient(version: &str) -> bool {
    // Parse version format YYYY.MM.DD
    let parts: Vec<&str> = version.split('.').collect();
    
    if parts.len() >= 3 {
        if let (Ok(year), Ok(month), Ok(day)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            // Minimum required version is 2022.01.21
            return (year > 2022) || 
                   (year == 2022 && month > 1) || 
                   (year == 2022 && month == 1 && day >= 21);
        }
    }
    
    // If we can't parse the version, assume it's not sufficient
    false
}

/// Check if ffmpeg version is sufficient
fn is_ffmpeg_version_sufficient(version: &str) -> bool {
    // Parse version like "4.2.7" or "4.4.1"
    let parts: Vec<&str> = version.split('.').collect();
    
    if parts.len() >= 2 {
        if let (Ok(major), Ok(_minor)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
        ) {
            // Minimum required version is 4.0
            return major >= 4;
        }
    }
    
    // If we can't parse the version, assume it's not sufficient
    false
}

/// Install or update a dependency
pub fn install_or_update_dependency(name: &str) -> Result<(), AppError> {
    match name {
        "yt-dlp" => install_ytdlp(),
        "ffmpeg" => install_ffmpeg(),
        _ => Err(AppError::General(format!("Unknown dependency: {}", name))),
    }
}

/// Install or update yt-dlp
fn install_ytdlp() -> Result<(), AppError> {
    println!("{}", "Installing/updating yt-dlp...".blue());
    
    let result = if cfg!(target_os = "windows") {
        // On Windows, we can use pip
        Command::new("pip")
            .args(["install", "--upgrade", "yt-dlp"])
            .status()
    } else if cfg!(target_os = "macos") {
        // On macOS, try homebrew first
        let brew_result = Command::new("brew")
            .args(["install", "yt-dlp"])
            .status();
            
        if brew_result.is_err() || !brew_result.as_ref().map(|s| s.success()).unwrap_or(false) {
            // Fall back to pip
            Command::new("pip3")
                .args(["install", "--user", "--upgrade", "yt-dlp"])
                .status()
        } else {
            brew_result
        }
    } else {
        // On Linux, try pip
        Command::new("pip3")
            .args(["install", "--user", "--upgrade", "yt-dlp"])
            .status()
    };
    
    match result {
        Ok(status) if status.success() => {
            println!("{}", "yt-dlp installed/updated successfully.".green());
            Ok(())
        },
        Ok(_) => {
            Err(AppError::General("yt-dlp installation failed".to_string()))
        },
        Err(e) => {
            Err(AppError::General(format!("Error installing yt-dlp: {}", e)))
        }
    }
}

/// Install or update ffmpeg
fn install_ffmpeg() -> Result<(), AppError> {
    println!("{}", "Installing ffmpeg...".blue());
    
    let result = if cfg!(target_os = "windows") {
        // On Windows, we need to guide the user
        println!("{}", "Automatic installation of ffmpeg is not supported on Windows.".yellow());
        println!("{}", "Please download and install ffmpeg manually from: https://ffmpeg.org/download.html".yellow());
        return Err(AppError::General("Manual installation required".to_string()));
    } else if cfg!(target_os = "macos") {
        // On macOS, use homebrew
        Command::new("brew")
            .args(["install", "ffmpeg"])
            .status()
    } else {
        // On Linux, use apt, assuming Debian/Ubuntu
        Command::new("sudo")
            .args(["apt", "install", "-y", "ffmpeg"])
            .status()
    };
    
    match result {
        Ok(status) if status.success() => {
            println!("{}", "ffmpeg installed successfully.".green());
            Ok(())
        },
        Ok(_) => {
            Err(AppError::General("ffmpeg installation failed".to_string()))
        },
        Err(e) => {
            Err(AppError::General(format!("Error installing ffmpeg: {}", e)))
        }
    }
}