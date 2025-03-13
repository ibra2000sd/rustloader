// src/license.rs

use crate::error::AppError;
use ring::digest;
use std::fs;
use std::path::PathBuf;
use dirs_next as dirs;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};

// License information structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LicenseInfo {
    pub license_key: String,
    pub user_email: String,
    pub activation_date: DateTime<Utc>,
    pub expiration_date: Option<DateTime<Utc>>,
    pub machine_id: String,
}

// License verification result
pub enum LicenseStatus {
    Free,
    Pro(LicenseInfo),
    Invalid(String), // Contains the reason for invalidity
}

// Get a unique machine identifier with simplified implementation
fn get_machine_id() -> Result<String, AppError> {
    // Start with hostname as the simplest cross-platform solution
    if let Ok(hostname) = hostname::get() {
        return Ok(hostname.to_string_lossy().to_string());
    }
    
    // Platform-specific fallbacks if hostname fails
    #[cfg(target_os = "linux")]
    {
        // Try machine-id on Linux
        if let Ok(id) = fs::read_to_string("/etc/machine-id") {
            return Ok(id.trim().to_string());
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // Use system_profiler on macOS as a fallback
        use std::process::Command;
        if let Ok(output) = Command::new("system_profiler")
            .arg("SPHardwareDataType")
            .output() {
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("Hardware UUID") {
                    if let Some(uuid) = line.split(":").nth(1) {
                        return Ok(uuid.trim().to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // Use ComputerName on Windows as a fallback
        use std::env;
        if let Ok(name) = env::var("COMPUTERNAME") {
            return Ok(name);
        }
    }
    
    // Generic fallback: timestamp + random number
    use rand::Rng;
    let timestamp = Utc::now().timestamp();
    let random_num = rand::thread_rng().gen::<u32>();
    Ok(format!("generic-{}-{}", timestamp, random_num))
}

// Path to the license file
fn get_license_path() -> Result<PathBuf, AppError> {
    let mut path = match dirs::config_dir() {
        Some(dir) => dir,
        None => {
            // Fall back to home directory if config_dir fails
            match dirs::home_dir() {
                Some(home) => home,
                None => {
                    // Last resort: use current directory
                    std::env::current_dir()
                        .map_err(|_| AppError::PathError("Could not find any valid configuration directory".to_string()))?
                }
            }
        }
    };
    
    path.push("rustloader");
    
    // Create directory if it doesn't exist, with better error handling
    if !path.exists() {
        match fs::create_dir_all(&path) {
            Ok(_) => println!("Created license directory: {:?}", path),
            Err(e) => {
                println!("Warning: Couldn't create license directory: {:?}. Error: {}", path, e);
                // Try to use temp directory as fallback
                path = std::env::temp_dir();
                path.push("rustloader");
                fs::create_dir_all(&path)?;
                println!("Using temporary license directory instead: {:?}", path);
            }
        }
    }
    
    path.push("license.dat");
    Ok(path)
}

// Simplified license verification
fn verify_license_key(license_key: &str) -> Result<bool, AppError> {
    // Basic format check
    if !license_key.starts_with("PRO-") || license_key.len() < 20 {
        return Ok(false);
    }
    
    // Check for valid license format
    let valid_chars = license_key.chars().all(|c| 
        c.is_ascii_alphanumeric() || c == '-' || c == '_'
    );
    
    if !valid_chars {
        return Ok(false);
    }
    
    // Check for expected segments
    let segments: Vec<&str> = license_key.split('-').collect();
    if segments.len() != 4 {
        return Ok(false);
    }
    
    // Simplified verification logic - just check the format
    // A real implementation would check with a license server
    Ok(true)
}

// Save license information to disk with better error handling
pub fn save_license(license: &LicenseInfo) -> Result<(), AppError> {
    let license_path = get_license_path()?;
    
    // Create a simple signature for the license data
    let license_data = serde_json::to_string(license)?;
    
    // Create a simple hash signature (for demo purposes only)
    let digest = digest::digest(&digest::SHA256, license_data.as_bytes());
    let signature = general_purpose::STANDARD.encode(digest.as_ref());
    
    // Combine license data and signature
    let full_data = format!("{}\n{}", license_data, signature);
    
    // Encode the data
    let encoded_data = general_purpose::STANDARD.encode(full_data.as_bytes());
    
    // Write to file with better error handling
    match fs::write(&license_path, &encoded_data) {
        Ok(_) => {
            println!("License saved successfully to {:?}", license_path);
            Ok(())
        }
        Err(e) => {
            // Try writing to temp directory as fallback
            let mut temp_path = std::env::temp_dir();
            temp_path.push("rustloader_license.dat");
            
            match fs::write(&temp_path, &encoded_data) {
                Ok(_) => {
                    println!("Warning: Couldn't write to primary license location. Saved to temp location instead: {:?}", temp_path);
                    Ok(())
                }
                Err(_) => {
                    Err(AppError::IoError(e))
                }
            }
        }
    }
}

// Load and verify license from disk with better error handling
pub fn load_license() -> Result<LicenseStatus, AppError> {
    let license_path = get_license_path()?;
    
    // If set in environment, override the license check for testing
    if let Ok(force_pro) = std::env::var("RUSTLOADER_FORCE_PRO") {
        if force_pro == "1" || force_pro.to_lowercase() == "true" {
            println!("Notice: Pro mode forced by environment variable for testing");
            let machine_id = get_machine_id()?;
            return Ok(LicenseStatus::Pro(LicenseInfo {
                license_key: "TEST-MODE".to_string(),
                user_email: "test@example.com".to_string(),
                activation_date: Utc::now(),
                expiration_date: None,
                machine_id,
            }));
        }
    }
    
    // Check if license file exists
    if !license_path.exists() {
        return Ok(LicenseStatus::Free);
    }
    
    // Try to read the license file
    let encoded_data = match fs::read_to_string(&license_path) {
        Ok(data) => data,
        Err(e) => {
            println!("Warning: Failed to read license file: {}", e);
            return Ok(LicenseStatus::Free);
        }
    };
    
    // Try to decode the license data
    let full_data = match general_purpose::STANDARD.decode(&encoded_data) {
        Ok(data) => {
            match String::from_utf8(data) {
                Ok(text) => text,
                Err(_) => {
                    println!("Warning: License file contains invalid data");
                    return Ok(LicenseStatus::Free);
                }
            }
        },
        Err(_) => {
            println!("Warning: License file is corrupted");
            return Ok(LicenseStatus::Free);
        }
    };
    
    // Split into license data and signature
    let parts: Vec<&str> = full_data.split('\n').collect();
    if parts.len() != 2 {
        println!("Warning: License file has invalid format");
        return Ok(LicenseStatus::Free);
    }
    
    let license_data = parts[0];
    
    // Parse license data with better error handling
    let license: LicenseInfo = match serde_json::from_str(license_data) {
        Ok(license) => license,
        Err(e) => {
            println!("Warning: Failed to parse license data: {}", e);
            return Ok(LicenseStatus::Free);
        }
    };
    
    // Check if license key itself is valid (only a simple check for this demo)
    if !verify_license_key(&license.license_key)? {
        return Ok(LicenseStatus::Invalid("License key format is invalid".to_string()));
    }
    
    // Check if license has expired
    if let Some(expiration) = license.expiration_date {
        if expiration < Utc::now() {
            return Ok(LicenseStatus::Invalid("License has expired".to_string()));
        }
    }
    
    // Simplified machine ID check
    // A real implementation would have more sophisticated validation
    // Just check that we're not on a completely different machine
    let current_machine_id = get_machine_id()?;
    
    // Only consider first 8 characters for more lenient matching
    // This helps with systems where machine ID might change slightly
    if license.machine_id.len() >= 8 && current_machine_id.len() >= 8 {
        let license_prefix = &license.machine_id[0..8];
        let current_prefix = &current_machine_id[0..8];
        
        if license_prefix != current_prefix {
            println!("Warning: Machine ID mismatch - license may have been transferred");
            // In this simplified version, we'll still allow it
            // A commercial implementation would be stricter
        }
    }
    
    Ok(LicenseStatus::Pro(license))
}

// Check if the current installation is Pro
pub fn is_pro_version() -> bool {
    match load_license() {
        Ok(LicenseStatus::Pro(_)) => true,
        _ => {
            // Check for environment override for testing
            if let Ok(force_pro) = std::env::var("RUSTLOADER_FORCE_PRO") {
                force_pro == "1" || force_pro.to_lowercase() == "true"
            } else {
                false
            }
        }
    }
}

// Activate a license key with better error handling
pub fn activate_license(license_key: &str, email: &str) -> Result<LicenseStatus, AppError> {
    // Basic validation
    if license_key.is_empty() {
        return Err(AppError::ValidationError("License key cannot be empty".to_string()));
    }
    
    if email.is_empty() {
        return Err(AppError::ValidationError("Email cannot be empty".to_string()));
    }
    
    // Basic email format check
    if !email.contains('@') || !email.contains('.') {
        return Err(AppError::ValidationError("Invalid email format".to_string()));
    }
    
    // Verify license key format
    if !verify_license_key(license_key)? {
        return Ok(LicenseStatus::Invalid("Invalid license key format".to_string()));
    }
    
    // Create new license info
    let license = LicenseInfo {
        license_key: license_key.to_string(),
        user_email: email.to_string(),
        activation_date: Utc::now(),
        expiration_date: None, // No expiration for this demo
        machine_id: get_machine_id()?,
    };
    
    // Save license to disk
    match save_license(&license) {
        Ok(_) => Ok(LicenseStatus::Pro(license)),
        Err(e) => {
            println!("Warning: Failed to save license: {}", e);
            
            // Even if we couldn't save it, return success for this session
            // and provide a warning to the user
            println!("Your license is valid but couldn't be saved permanently.");
            println!("You may need to activate it again next time.");
            
            Ok(LicenseStatus::Pro(license))
        }
    }
}

// Function to display license information with better formatting
pub fn display_license_info() -> Result<(), AppError> {
    match load_license()? {
        LicenseStatus::Free => {
            println!("License: {} Version", "Free".cyan());
            println!("---------------------------------------");
            println!("• Maximum video quality: 720p");
            println!("• Daily download limit: 5 videos");
            println!("• Audio quality: 128Kbps MP3");
            println!("---------------------------------------");
            println!("Upgrade to Pro: rustloader.com/pro");
        },
        LicenseStatus::Pro(license) => {
            println!("License: {} Version", "Pro".bright_green());
            println!("---------------------------------------");
            println!("• Email: {}", license.user_email);
            println!("• Activated: {}", license.activation_date.with_timezone(&Local).format("%Y-%m-%d"));
            if let Some(exp) = license.expiration_date {
                println!("• Expires: {}", exp.with_timezone(&Local).format("%Y-%m-%d"));
            } else {
                println!("• License Type: Perpetual (No Expiration)");
            }
            println!("---------------------------------------");
            println!("• Unlimited downloads");
            println!("• 4K/8K video quality");
            println!("• High-quality audio formats");
            println!("• Multi-threaded downloads");
            println!("• Priority support");
        },
        LicenseStatus::Invalid(reason) => {
            println!("License: {}", "Invalid".red());
            println!("Reason: {}", reason);
            println!("Reverting to Free Version");
            println!("---------------------------------------");
            println!("To reactivate or purchase a license, please visit:");
            println!("rustloader.com/account");
        }
    }
    
    Ok(())
}