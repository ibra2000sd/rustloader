// src/utils.rs

use crate::error::AppError;
use colored::*;
use home::home_dir;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ShellCommand, Stdio};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use semver::Version;
use ring::signature;
use base64::{Engine as _, engine::general_purpose};

/// Validate path to prevent path traversal attacks
pub fn validate_path_safety(path: &Path) -> Result<(), AppError> {
    // Canonicalize the path to resolve any .. or symlinks
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If path doesn't exist yet, we need to check its components
            return check_path_components(path);
        }
    };
    
    // Get user's home directory for comparison
    let home_dir = match home_dir() {
        Some(dir) => dir,
        None => return Err(AppError::PathError("Could not determine home directory".to_string())),
    };
    
    // Get the canonical form of the home directory
    let canonical_home = match home_dir.canonicalize() {
        Ok(h) => h,
        Err(_) => return Err(AppError::PathError("Could not canonicalize home directory".to_string())),
    };
    
    // Get download directory (should be under home)
    let mut downloads_dir = home_dir.clone();
    downloads_dir.push("Downloads");
    
    // Check if the provided path is within allowed directories
    if !canonical_path.starts_with(&canonical_home) && 
       !canonical_path.starts_with("/tmp") && 
       !canonical_path.starts_with("/var/tmp") {
        return Err(AppError::SecurityViolation);
    }
    
    // List of sensitive directories that should be avoided
    let sensitive_dirs = [
        "/etc", "/bin", "/sbin", "/usr/bin", "/usr/sbin",
        "/usr/local/bin", "/usr/local/sbin", "/var/run",
        "/boot", "/dev", "/proc", "/sys", "/var/log"
    ];
    
    // Convert to string for easy comparison
    let path_str = canonical_path.to_string_lossy().to_string();
    
    // Check if path contains any sensitive directories
    for dir in sensitive_dirs.iter() {
        if path_str.starts_with(dir) {
            return Err(AppError::SecurityViolation);
        }
    }
    
    Ok(())
}

/// Check path components for relative traversal attempts
fn check_path_components(path: &Path) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    
    // Check for potential path traversal sequences
    if path_str.contains("../") || path_str.contains("..\\") || 
       path_str.contains("/..") || path_str.contains("\\..") ||
       path_str.contains("~") {
        return Err(AppError::SecurityViolation);
    }
    
    // Check each component
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // Attempting to navigate up - potential path traversal
                return Err(AppError::SecurityViolation);
            },
            _ => continue,
        }
    }
    
    Ok(())
}

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

/// Validate a URL format with enhanced security
pub fn validate_url(url: &str) -> Result<(), AppError> {
    // More comprehensive URL validation
    let url_regex = Regex::new(r"^https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&//=]*)$").unwrap();
    
    // Check for common URLs we want to support
    let youtube_regex = Regex::new(r"^https?://(?:www\.)?(?:youtube\.com|youtu\.be)/").unwrap();
    let vimeo_regex = Regex::new(r"^https?://(?:www\.)?vimeo\.com/").unwrap();
    let dailymotion_regex = Regex::new(r"^https?://(?:www\.)?dailymotion\.com/").unwrap();
    
    if !(url_regex.is_match(url) || youtube_regex.is_match(url) || vimeo_regex.is_match(url) || dailymotion_regex.is_match(url)) {
        return Err(AppError::ValidationError(format!("Invalid URL format: {}", url)));
    }
    
    // Security check: Prevent command injection attempts in URLs
    if url.contains("&&") || url.contains(";") || url.contains("|") || url.contains("`") {
        return Err(AppError::SecurityViolation);
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

/// Validate the provided bitrate format (e.g., 1000K)
pub fn validate_bitrate(bitrate: &str) -> Result<(), AppError> {
    // Enhanced regex check for format like "1000K" or "5M"
    let re = Regex::new(r"^(\d+)(K|M)$").unwrap();
    
    if !re.is_match(bitrate) {
        return Err(AppError::ValidationError(
            format!("Invalid bitrate format: {}. Use format like '1000K' or '5M'", bitrate)
        ));
    }
    
    // Extract and check numerical value
    if let Some(captures) = re.captures(bitrate) {
        let value = captures.get(1).unwrap().as_str();
        let value_num: u32 = match value.parse() {
            Ok(num) => num,
            Err(_) => {
                return Err(AppError::ValidationError(
                    format!("Invalid bitrate value: {}. Must be a valid number.", value)
                ));
            }
        };
        
        // Set reasonable limits
        if value_num == 0 {
            return Err(AppError::ValidationError("Bitrate cannot be zero.".to_string()));
        }
        
        let unit = captures.get(2).unwrap().as_str();
        if unit == "K" && value_num > 10000 {
            return Err(AppError::ValidationError("Bitrate too high (max 10000K)".to_string()));
        } else if unit == "M" && value_num > 100 {
            return Err(AppError::ValidationError("Bitrate too high (max 100M)".to_string()));
        }
    }
    
    Ok(())
}

/// Enhanced initialize_download_dir with security checks
pub fn initialize_download_dir(
    custom_dir: Option<&str>, 
    program_name: &str, 
    file_type: &str,
) -> Result<PathBuf, AppError> {
    let download_dir = if let Some(dir) = custom_dir {
        // Create path from provided directory
        let path = PathBuf::from(dir);
        
        // Validate path safety
        validate_path_safety(&path)?;
        
        path
    } else {
        // Use default path under home directory
        match home_dir() {
            Some(mut path) => {
                path.push("Downloads");
                path.push(program_name);
                path.push(file_type);
                
                // Still validate the default path to be sure
                validate_path_safety(&path)?;
                
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

/// Sanitize a path string using a strict whitelist approach
fn sanitize_path(path: &str) -> Result<String, AppError> {
    // Split path into directory and filename components
    let path_obj = std::path::Path::new(path);
    
    // Process the directory part
    let dir_part = if let Some(parent) = path_obj.parent() {
        let dir_str = parent.to_string_lossy();
        
        // Validate directory - more permissive but still safe
        if dir_str.contains("..") || 
           dir_str.contains('~') || 
           dir_str.contains('*') || 
           dir_str.contains('?') || 
           dir_str.contains('|') ||
           dir_str.contains(';') || 
           dir_str.contains('&') ||
           dir_str.contains('<') || 
           dir_str.contains('>') {
            return Err(AppError::ValidationError("Directory path contains invalid characters".to_string()));
        }
        
        dir_str.to_string()
    } else {
        String::new()
    };
    
    // Process the filename part with stricter validation
    let file_part = if let Some(file_name) = path_obj.file_name() {
        let file_str = file_name.to_string_lossy();
        
        // Stricter whitelist for filenames
        let sanitized_file: String = file_str.chars()
            .filter(|c| c.is_ascii_alphanumeric() || 
                         *c == '.' || *c == '-' || *c == '_' || 
                         *c == ' ' || *c == '(' || *c == ')' ||
                         // Allow variable placeholder for yt-dlp
                         *c == '%')
            .collect();
        
        // If too many characters were filtered out, reject it
        if sanitized_file.len() < file_str.len() * 3 / 4 {
            return Err(AppError::ValidationError("Filename contains too many invalid characters".to_string()));
        }
        
        sanitized_file
    } else {
        return Err(AppError::ValidationError("No filename in path".to_string()));
    };
    
    // Recombine path with proper separator
    if dir_part.is_empty() {
        Ok(file_part)
    } else {
        Ok(format!("{}/{}", dir_part, file_part))
    }
}

/// Format a safe path for use with yt-dlp
pub fn format_output_path<P: AsRef<Path>>(
    download_dir: P, 
    format: &str
) -> Result<String, AppError> {
    // Additional validation
    validate_path_safety(download_dir.as_ref())?;
    
    // Make sure the format is valid
    match format {
        "mp3" | "mp4" | "webm" | "m4a" | "flac" | "wav" | "ogg" => {},
        _ => return Err(AppError::ValidationError(format!("Invalid output format: {}", format))),
    }
    
    let path_str = download_dir
        .as_ref()
        .join(format!("%(title)s.{}", format))
        .to_str()
        .ok_or_else(|| AppError::PathError("Invalid path encoding".to_string()))?
        .to_string();
    
    // Sanitize the path to remove potentially dangerous characters
    let sanitized_path = sanitize_path(&path_str)?;
    
    Ok(sanitized_path)
}

// Enhanced version info structure with signature
#[derive(Deserialize, Debug)]
struct SignedReleaseInfo {
    release: ReleaseInfo,
    signature: String,
    pub_key_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReleaseInfo {
    tag_name: String,
    html_url: String,
    prerelease: bool,
    release_notes: String,
    release_date: String,
    checksum: String,
}

// Public keys for trusted developers (in a real app, this would be more securely managed)
struct TrustedKeys {
    keys: Vec<(String, Vec<u8>)>,
}

impl TrustedKeys {
    fn new() -> Self {
        Self {
            keys: vec![
                // Example key ID and public key (base64 encoded)
                // In a real implementation, these would be real RSA/ECDSA public keys
                (
                    "rustloader-release-key-1".to_string(), 
                    general_purpose::STANDARD.decode(
                        "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzm8X3PIzQAHU0QN9JV9TOT+1F5iHnJXUm" 
                    ).unwrap_or_default()
                ),
                // Additional trusted keys could be added here
            ],
        }
    }
    
    fn get_key_by_id(&self, key_id: &str) -> Option<&Vec<u8>> {
        self.keys.iter()
            .find(|(id, _)| id == key_id)
            .map(|(_, key)| key)
    }
}

/// Verify the signature of release data
fn verify_release_signature(data: &ReleaseInfo, signature: &str, public_key: &[u8]) -> Result<bool, AppError> {
    // Convert the release data to a JSON string for signature verification
    let data_json = serde_json::to_string(data)?;
    
    // Decode the signature from base64
    let signature_bytes = match general_purpose::STANDARD.decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };
    
    // Verify the signature
    // Note: This is a simplified example. In a real implementation,
    // you would use an actual asymmetric signature verification algorithm.
    match verify_signature(&data_json.as_bytes(), &signature_bytes, public_key) {
        Ok(valid) => Ok(valid),
        Err(_) => Ok(false),
    }
}

/// Simplified signature verification function
/// In a real implementation, this would use proper cryptographic signatures
fn verify_signature(data: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, AppError> {
    // For demonstration purposes, we'll use ring's ECDSA verification
    // In a real app, you'd use the appropriate algorithm based on your key type
    let public_key = signature::UnparsedPublicKey::new(
        &signature::ECDSA_P256_SHA256_ASN1,
        public_key
    );
    
    match public_key.verify(data, signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Securely check for updates with signature verification
pub async fn check_for_updates() -> Result<bool, AppError> {
    // Current version from the code
    let current_version = match Version::parse(crate::VERSION) {
        Ok(v) => v,
        Err(_) => return Err(AppError::General("Invalid current version format".to_string())),
    };
    
    // Create a client with a timeout and HTTPS enforcement
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .https_only(true) // Enforce HTTPS for security
        .build()?;
    
    // URL for the update server API
    let url = "https://api.rustloader.com/releases/latest";
    
    // Make the request with proper headers
    let response = match client.get(url)
        .header("User-Agent", format!("rustloader/{}", crate::VERSION))
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                // Don't fail if we can't check for updates, just log and continue
                println!("{} {}", "Could not check for updates:".yellow(), e);
                return Ok(false)
            }
        };
    
    // Parse the response with signature data
    if response.status().is_success() {
        match response.json::<SignedReleaseInfo>().await {
            Ok(signed_release) => {
                // Skip prereleases for general users
                if signed_release.release.prerelease {
                    return Ok(false);
                }
                
                // Verify the signature
                let trusted_keys = TrustedKeys::new();
                if let Some(public_key) = trusted_keys.get_key_by_id(&signed_release.pub_key_id) {
                    let signature_valid = verify_release_signature(
                        &signed_release.release,
                        &signed_release.signature,
                        public_key
                    )?;
                    
                    if !signature_valid {
                        println!("{}", "Update signature verification failed!".red());
                        return Ok(false);
                    }
                } else {
                    println!("{}", "Update signed with untrusted key!".red());
                    return Ok(false);
                }
                
                // Parse version from tag (remove 'v' prefix if present)
                let version_str = signed_release.release.tag_name.trim_start_matches('v');
                match Version::parse(version_str) {
                    Ok(latest_version) => {
                        // Compare versions
                        if latest_version > current_version {
                            println!("{} {} -> {}", 
                                "New version available:".bright_yellow(),
                                current_version,
                                latest_version
                            );
                            println!("{} {}", "Download at:".bright_yellow(), signed_release.release.html_url);
                            println!("{} {}", "Release notes:".bright_cyan(), signed_release.release.release_notes);
                            println!("{} {}", "SHA-256 checksum:".bright_cyan(), signed_release.release.checksum);
                            return Ok(true);
                        }
                    },
                    Err(_) => {
                        // If we can't parse the version, assume no update
                        return Ok(false);
                    }
                }
            },
            Err(_) => {
                // If we can't parse the response, assume no update
                return Ok(false);
            }
        }
    }
    
    // No update available or couldn't determine
    Ok(false)
}
