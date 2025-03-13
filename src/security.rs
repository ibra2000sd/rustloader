// src/security.rs
// Simplified security module with less restrictive validation

use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;

// Rate limiting state
static RATE_LIMITS: Lazy<Mutex<HashMap<String, Vec<Instant>>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Initialize the security module - simple version
pub fn init() {
    // Check if we're in development mode
    if cfg!(debug_assertions) {
        println!("Security module initialized in development mode (relaxed checks)");
    } else {
        println!("Security module initialized in production mode");
    }
}

/// Apply rate limiting to security-sensitive operations
pub fn apply_rate_limit(operation: &str, max_attempts: usize, window: Duration) -> bool {
    // Skip rate limiting in development mode
    if cfg!(debug_assertions) {
        return true;
    }

    let now = Instant::now();
    
    // Try to get the mutex lock
    let mut limits = match RATE_LIMITS.lock() {
        Ok(guard) => guard,
        Err(_) => {
            // If we can't acquire the lock, default to allowing the operation
            eprintln!("WARNING: Could not acquire rate limit lock");
            return true;
        }
    };
    
    // Get or create the entry for this operation
    let attempts = limits.entry(operation.to_string()).or_insert_with(Vec::new);
    
    // Remove attempts outside the time window
    attempts.retain(|time| now.duration_since(*time) < window);
    
    // Check if we've exceeded the limit
    if attempts.len() >= max_attempts {
        eprintln!("Rate limit exceeded for operation: {}", operation);
        return false;
    }
    
    // Add this attempt
    attempts.push(now);
    true
}

/// Validate a path with more lenient rules
pub fn validate_path_safety(path: &Path) -> Result<(), AppError> {
    // Skip validation in development mode
    if cfg!(debug_assertions) {
        return Ok(());
    }

    // Basic path validation
    let path_str = path.to_string_lossy();
    
    // Check path length (very large paths could indicate an attack)
    if path_str.len() > 4096 {
        return Err(AppError::ValidationError(
            "Path exceeds maximum allowed length".to_string()
        ));
    }
    
    // Check for null bytes (definitely an attack)
    if path_str.contains('\0') {
        return Err(AppError::SecurityViolation);
    }
    
    // Check for obvious path traversal attempts
    if path_str.contains("../..") || path_str.contains("..\\..") {
        return Err(AppError::SecurityViolation);
    }
    
    // For absolute paths, check they're not sensitive system directories
    if path.is_absolute() {
        let sensitive_dirs = [
            "/etc", "/bin", "/sbin", "/usr/bin", "/usr/sbin",
            "C:\\Windows\\System32", "C:\\Windows", "C:\\Program Files",
        ];
        
        for dir in &sensitive_dirs {
            if path_str.starts_with(dir) {
                return Err(AppError::ValidationError(
                    format!("Access to system directory '{}' is not allowed", dir)
                ));
            }
        }
    }
    
    // Path seems safe
    Ok(())
}

/// Validate URL format with security checks
pub fn validate_url(url: &str) -> Result<(), AppError> {
    // Skip validation in development mode
    if cfg!(debug_assertions) {
        if url.starts_with("http://") || url.starts_with("https://") {
            return Ok(());
        }
    }
    
    // Apply rate limiting to URL validation to prevent DoS
    if !apply_rate_limit("url_validation", 20, Duration::from_secs(60)) {
        return Err(AppError::ValidationError(
            "Too many validation attempts. Please try again later.".to_string()
        ));
    }
    
    // Check URL length
    if url.len() > 2048 {
        return Err(AppError::ValidationError(
            "URL exceeds maximum allowed length".to_string()
        ));
    }
    
    // Basic URL validation - must start with http:// or https://
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::ValidationError(
            "URL must start with http:// or https://".to_string()
        ));
    }
    
    // Allow common video platform URLs with specific patterns
    let video_platforms = [
        "youtube.com", "youtu.be", "vimeo.com", "dailymotion.com",
        "facebook.com", "twitter.com", "instagram.com", "tiktok.com"
    ];
    
    // Check if URL contains any of the allowed platforms
    if video_platforms.iter().any(|platform| url.contains(platform)) {
        return Ok(());
    }
    
    // For other URLs, use a more general regex check
    let url_regex = Regex::new(r"^https?://(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?(?:/[^\s]*)?$").unwrap();
    
    if url_regex.is_match(url) {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!("Invalid URL format: {}", url)))
    }
}

/// Sanitize a string to help prevent command injection
pub fn sanitize_shell_arg(arg: &str) -> String {
    // A very simple implementation that works for most common cases
    // In a real production environment, you'd want something more robust
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, wrap in quotes and escape internal quotes
        let escaped = arg.replace("\"", "\\\"");
        format!("\"{}\"", escaped)
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // On Unix, wrap in single quotes
        // Single quotes inside the string need special handling
        if !arg.contains('\'') {
            format!("'{}'", arg)
        } else {
            // Use this pattern: 'string with '\''quote'\'' in it'
            let escaped = arg.replace('\'', "'\\''");
            format!("'{}'", escaped)
        }
    }
}

/// Create a secure temporary directory with proper permissions
pub fn create_secure_temp_dir(prefix: &str) -> Result<PathBuf, AppError> {
    let temp_dir = std::env::temp_dir();
    let dir_name = format!("{}_{}", prefix, std::process::id()); // Use process ID for uniqueness
    let dir_path = temp_dir.join(dir_name);
    
    // Create the directory with appropriate permissions
    match fs::create_dir_all(&dir_path) {
        Ok(_) => {
            // On Unix systems, set proper permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = fs::metadata(&dir_path) {
                    let mut perms = metadata.permissions();
                    // 0o700 = owner read/write/execute only
                    perms.set_mode(0o700);
                    let _ = fs::set_permissions(&dir_path, perms);
                }
            }
            
            Ok(dir_path)
        },
        Err(e) => Err(AppError::IoError(e))
    }
}