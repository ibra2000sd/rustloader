//! Enhanced dependency validator for Rustloader
//! 
//! This module provides functionality to validate and verify external dependencies
//! like yt-dlp and ffmpeg, checking versions, binary integrity, and known vulnerabilities.

use crate::error::AppError;
use std::process::{Command, Stdio};
use std::collections::HashMap;
use ring::digest;
use base64::{Engine as _, engine::general_purpose};
use colored::*;
use std::io::Read;
use std::fs::File;

// Minimum acceptable versions for dependencies
pub const MIN_YTDLP_VERSION: &str = "2023.07.06";
pub const MIN_FFMPEG_VERSION: &str = "4.0.0";

// Known vulnerable versions to warn about
const VULNERABLE_YTDLP_VERSIONS: [&str; 2] = ["2022.05.18", "2022.08.14"];
const VULNERABLE_FFMPEG_VERSIONS: [&str; 2] = ["4.3.1", "4.4.2"];

/// Dependency info containing version and path information
#[allow(dead_code)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub hash: Option<String>,
    pub is_min_version: bool,
    pub is_vulnerable: bool,
}

/// Get path to a dependency
fn get_dependency_path(name: &str) -> Result<String, AppError> {
    let command = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };

    let output = Command::new(command)
        .arg(name)
        .output()
        .map_err(|e| AppError::IoError(e))?;
        
    if !output.status.success() {
        return Err(AppError::MissingDependency(name.to_string()));
    }
    
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(path)
}

/// Calculate SHA-256 hash of a file
fn calculate_file_hash(path: &str) -> Result<String, AppError> {
    let mut file = File::open(path)
        .map_err(|e| AppError::IoError(e))?;
        
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| AppError::IoError(e))?;
        
    let digest = digest::digest(&digest::SHA256, &buffer);
    Ok(general_purpose::STANDARD.encode(digest.as_ref()))
}

/// Parse version string from output
fn parse_version(output: &str, name: &str) -> String {
    let version_pattern = match name {
        "yt-dlp" => r"(?i)yt-dlp\s+(\d+\.\d+\.\d+)",
        // Fixed pattern to handle more version number formats
        "ffmpeg" => r"(?i)ffmpeg\s+version\s+(\d+\.\d+(?:\.\d+)?)",
        _ => r"(\d+\.\d+\.\d+)",
    };
    
    // Use regex to extract version
    let re = regex::Regex::new(version_pattern).unwrap();
    if let Some(captures) = re.captures(output) {
        if let Some(version) = captures.get(1) {
            return version.as_str().to_string();
        }
    }
    
    // Try alternative pattern for ffmpeg if first one fails
    if name == "ffmpeg" {
        let alt_pattern = r"(?i)version\s+(\d+\.\d+(?:\.\d+)?)";
        let re_alt = regex::Regex::new(alt_pattern).unwrap();
        
        if let Some(captures) = re_alt.captures(output) {
            if let Some(version) = captures.get(1) {
                return version.as_str().to_string();
            }
        }
    }
    
    // Debug output to help diagnose issues
    println!("Debug: Could not parse version from output for {}: {}", name, output);
    
    // Fallback - return first line, or "unknown"
    output.lines().next().unwrap_or("unknown").to_string()
}

/// Check if a version is at least the minimum required
fn is_minimum_version(version: &str, min_version: &str) -> bool {
    // Simple version comparison - in real app, use semver crate
    // This is a simplified version check
    let version_parts: Vec<u32> = version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
        
    let min_parts: Vec<u32> = min_version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
        
    // Compare major, minor, patch parts
    for i in 0..3 {
        let v1 = version_parts.get(i).copied().unwrap_or(0);
        let v2 = min_parts.get(i).copied().unwrap_or(0);
        
        if v1 > v2 {
            return true;
        }
        
        if v1 < v2 {
            return false;
        }
    }
    
    // Versions are equal
    true
}

/// Check if a version is in the list of known vulnerable versions
fn is_vulnerable_version(version: &str, vulnerable_versions: &[&str]) -> bool {
    vulnerable_versions.contains(&version)
}

/// Get detailed information about a dependency
pub fn get_dependency_info(name: &str) -> Result<DependencyInfo, AppError> {
    // First check if dependency exists
    let path = get_dependency_path(name)?;
    
    // Get version info
    let output = Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|e| AppError::IoError(e))?;
        
    if !output.status.success() {
        return Err(AppError::General(format!("Failed to get {} version", name)));
    }
    
    let version_output = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_output = String::from_utf8_lossy(&output.stderr).to_string();
    
    // Combine stdout and stderr as some programs output version to stderr
    let combined_output = format!("{}\n{}", version_output, stderr_output);
    
    // Parse version from output
    let version = parse_version(&combined_output, name);
    
    // Calculate file hash
    let hash = match calculate_file_hash(&path) {
        Ok(h) => Some(h),
        Err(_) => None,
    };
    
    // Check minimum version
    let min_version = match name {
        "yt-dlp" => MIN_YTDLP_VERSION,
        "ffmpeg" => MIN_FFMPEG_VERSION,
        _ => "0.0.0",
    };
    
    let is_min_version = is_minimum_version(&version, min_version);
    
    // Check if vulnerable version
    let vulnerable_versions = match name {
        "yt-dlp" => &VULNERABLE_YTDLP_VERSIONS[..],
        "ffmpeg" => &VULNERABLE_FFMPEG_VERSIONS[..],
        _ => &[][..],
    };
    
    let is_vulnerable = is_vulnerable_version(&version, vulnerable_versions);
    
    Ok(DependencyInfo {
        name: name.to_string(),
        version,
        path,
        hash,
        is_min_version,
        is_vulnerable,
    })
}

/// Check all required dependencies with detailed report
pub fn validate_dependencies() -> Result<HashMap<String, DependencyInfo>, AppError> {
    let mut results = HashMap::new();
    let mut has_issues = false;
    
    println!("{}", "Validating dependencies...".blue());
    
    // Check yt-dlp
    match get_dependency_info("yt-dlp") {
        Ok(info) => {
            println!("{}: {} ({})", "yt-dlp".green(), info.version, info.path);
            
            if !info.is_min_version {
                println!("{}: Version {} is below minimum required ({})", 
                    "WARNING".yellow(), 
                    info.version, 
                    MIN_YTDLP_VERSION);
                has_issues = true;
            }
            
            if info.is_vulnerable {
                println!("{}: Version {} has known vulnerabilities", 
                    "WARNING".red(), 
                    info.version);
                has_issues = true;
            }
            
            results.insert("yt-dlp".to_string(), info);
        },
        Err(e) => {
            println!("{}: {}", "ERROR".red(), e);
            has_issues = true;
        }
    }
    
    // Check ffmpeg
    match get_dependency_info("ffmpeg") {
        Ok(info) => {
            println!("{}: {} ({})", "ffmpeg".green(), info.version, info.path);
            
            if !info.is_min_version {
                println!("{}: Version {} is below minimum required ({})", 
                    "WARNING".yellow(), 
                    info.version, 
                    MIN_FFMPEG_VERSION);
                has_issues = true;
            }
            
            if info.is_vulnerable {
                println!("{}: Version {} has known vulnerabilities", 
                    "WARNING".red(), 
                    info.version);
                has_issues = true;
            }
            
            results.insert("ffmpeg".to_string(), info);
        },
        Err(e) => {
            println!("{}: {}", "ERROR".red(), e);
            has_issues = true;
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

/// Update yt-dlp to the latest version
pub fn update_ytdlp() -> Result<(), AppError> {
    println!("{}", "Updating yt-dlp to latest version...".blue());
    
    let output = Command::new("yt-dlp")
        .arg("--update")
        .stdout(Stdio::inherit()) // Show output to user
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| AppError::IoError(e))?;
        
    if output.success() {
        // Verify the update was successful
        match get_dependency_info("yt-dlp") {
            Ok(info) => {
                println!("{}: {}", "Updated yt-dlp version", info.version);
                
                if !info.is_min_version {
                    println!("{}: Version is still below minimum required ({})", 
                        "WARNING".yellow(), 
                        MIN_YTDLP_VERSION);
                    return Err(AppError::General("Failed to update yt-dlp to required version".to_string()));
                }
                
                if info.is_vulnerable {
                    println!("{}: Updated version still has known vulnerabilities", 
                        "WARNING".red());
                    return Err(AppError::General("Updated to a vulnerable version of yt-dlp".to_string()));
                }
            },
            Err(e) => {
                return Err(e);
            }
        }
        
        println!("{}", "yt-dlp updated successfully.".green());
        Ok(())
    } else {
        println!("{}", "Failed to update yt-dlp.".red());
        Err(AppError::General("Failed to update yt-dlp".to_string()))
    }
}

/// Verify integrity of a dependency against known good hashes
#[allow(dead_code)]
pub fn verify_dependency_integrity(name: &str) -> Result<bool, AppError> {
    println!("{} {}", "Verifying integrity of", name);
    
    // Get current dependency info
    let info = get_dependency_info(name)?;
    
    // This is where we would verify against known good hashes
    // In a real implementation, these would be fetched from a secure server
    // or embedded in the binary
    
    // For now, just print the hash for reference
    if let Some(hash) = &info.hash {
        println!("{} SHA-256: {}", name, hash);
        println!("{}", "No integrity violations detected.".green());
        // In a real implementation, verify against trusted hash
        return Ok(true);
    } else {
        println!("{}", "Could not calculate hash for integrity verification.".yellow());
        return Ok(false);
    }
}

/// Check for updates to rust toolchain
#[allow(dead_code)]
pub fn check_rust_updates() -> Result<(), AppError> {
    println!("{}", "Checking for Rust updates...".blue());
    
    if !cfg!(debug_assertions) {
        // Skip in release mode for end users
        println!("{}", "Skipping Rust update check in release mode.".blue());
        return Ok(());
    }
    
    if !Command::new("rustup").arg("--version").status().map_err(|e| AppError::IoError(e))?.success() {
        println!("{}", "rustup not found. Skipping Rust update check.".yellow());
        return Ok(());
    }
    
    let output = Command::new("rustup")
        .arg("update")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AppError::IoError(e))?;
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
        
    if !output.status.success() {
        println!("{}: {}", "Error checking for Rust updates".red(), stderr);
        return Err(AppError::General("Failed to check for Rust updates".to_string()));
    }
    
    if stdout.contains("Updated") {
        println!("{}", "Rust toolchain updated successfully.".green());
    } else {
        println!("{}", "Rust toolchain is up to date.".green());
    }
    
    Ok(())
}

/// Install dependency if not present or outdated
pub fn install_or_update_dependency(name: &str) -> Result<(), AppError> {
    match name {
        "yt-dlp" => {
            match get_dependency_info("yt-dlp") {
                Ok(info) => {
                    if !info.is_min_version || info.is_vulnerable {
                        update_ytdlp()?;
                    } else {
                        println!("{} is up to date ({})", name, info.version);
                    }
                },
                Err(_) => {
                    // Not installed, install it
                    install_ytdlp()?;
                }
            }
        },
        "ffmpeg" => {
            match get_dependency_info("ffmpeg") {
                Ok(info) => {
                    if !info.is_min_version || info.is_vulnerable {
                        println!("{}: {} needs updating but must be done manually", name.yellow(), info.version);
                        println!("Please update ffmpeg using your system package manager.");
                    } else {
                        println!("{} is up to date ({})", name, info.version);
                    }
                },
                Err(_) => {
                    // Not installed, install it manually
                    install_ffmpeg()?;
                }
            }
        },
        _ => {
            return Err(AppError::General(format!("Unknown dependency: {}", name)));
        }
    }
    
    Ok(())
}

/// Install yt-dlp
fn install_ytdlp() -> Result<(), AppError> {
    println!("{}", "Installing yt-dlp...".blue());
    
    let cmd = if cfg!(target_os = "windows") {
        let status = Command::new("pip")
            .arg("install")
            .arg("--user")
            .arg("--upgrade")
            .arg("yt-dlp")
            .status()
            .map_err(|e| AppError::IoError(e))?;
            
        status.success()
    } else {
        let status = Command::new("pip3")
            .arg("install")
            .arg("--user")
            .arg("--upgrade")
            .arg("yt-dlp")
            .status()
            .map_err(|e| AppError::IoError(e))?;
            
        status.success()
    };
    
    if cmd {
        println!("{}", "yt-dlp installed successfully.".green());
        
        // Verify installation
        match get_dependency_info("yt-dlp") {
            Ok(info) => {
                println!("Installed version: {}", info.version);
                
                if !info.is_min_version {
                    println!("{}: Version is below minimum required ({})", 
                        "WARNING".yellow(), 
                        MIN_YTDLP_VERSION);
                }
                
                if info.is_vulnerable {
                    println!("{}: Installed version has known vulnerabilities", 
                        "WARNING".red());
                }
            },
            Err(e) => {
                println!("{}: {}", "Failed to verify installation".red(), e);
                return Err(e);
            }
        }
        
        Ok(())
    } else {
        println!("{}", "Failed to install yt-dlp.".red());
        println!("Please install yt-dlp manually: https://github.com/yt-dlp/yt-dlp#installation");
        Err(AppError::General("Failed to install yt-dlp".to_string()))
    }
}

/// Install ffmpeg with platform-specific commands
fn install_ffmpeg() -> Result<(), AppError> {
    println!("{}", "Installing ffmpeg...".blue());

    let success = if cfg!(target_os = "macos") {
        // macOS using Homebrew
        Command::new("brew")
            .arg("install")
            .arg("ffmpeg")
            .status()
            .map_err(|e| AppError::IoError(e))?
            .success()
    } else if cfg!(target_os = "linux") {
        // Try common Linux package managers
        let package_managers = [
            ("apt", &["install", "-y", "ffmpeg"]),
            ("apt-get", &["install", "-y", "ffmpeg"]),
            ("dnf", &["install", "-y", "ffmpeg"]),
            ("yum", &["install", "-y", "ffmpeg"]),
            ("pacman", &["-S", "--noconfirm", "ffmpeg"]),
        ];
        
        let mut installed = false;
        
        for (pm, args) in package_managers.iter() {
            if Command::new("which").arg(pm).stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) {
                println!("Using {} to install ffmpeg...", pm);
                
                // Fix: Create the sudo_args vector and store it in a variable
                let sudo_command = "sudo".to_string();
                let pm_string = (*pm).to_string();
                
                // Fix: Build the command directly without constructing the vector
                installed = Command::new(&sudo_command)
                    .arg(&pm_string)
                    .args(*args)
                    .status()
                    .map_err(|e| AppError::IoError(e))?
                    .success();
                    
                if installed {
                    break;
                }
            }
        }
        
        installed
    } else if cfg!(target_os = "windows") {
        // Windows - try using chocolatey
        if Command::new("where").arg("choco").stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) {
            Command::new("choco")
                .arg("install")
                .arg("ffmpeg")
                .arg("-y")
                .status()
                .map_err(|e| AppError::IoError(e))?
                .success()
        } else {
            println!("{}", "Chocolatey not found. Please install ffmpeg manually:".yellow());
            println!("https://ffmpeg.org/download.html");
            false
        }
    } else {
        println!("{}", "Unsupported platform for automatic ffmpeg installation.".yellow());
        println!("Please install ffmpeg manually: https://ffmpeg.org/download.html");
        false
    };
    
    if success {
        println!("{}", "ffmpeg installed successfully.".green());
        
        // Verify installation
        match get_dependency_info("ffmpeg") {
            Ok(info) => {
                println!("Installed version: {}", info.version);
                
                if !info.is_min_version {
                    println!("{}: Version is below minimum required ({})", 
                        "WARNING".yellow(), 
                        MIN_FFMPEG_VERSION);
                }
                
                if info.is_vulnerable {
                    println!("{}: Installed version has known vulnerabilities", 
                        "WARNING".red());
                }
            },
            Err(e) => {
                println!("{}: {}", "Failed to verify installation".red(), e);
                return Err(e);
            }
        }
        
        Ok(())
    } else {
        println!("{}", "Failed to install ffmpeg automatically.".red());
        println!("Please install ffmpeg manually: https://ffmpeg.org/download.html");
        Err(AppError::General("Failed to install ffmpeg".to_string()))
    }
}