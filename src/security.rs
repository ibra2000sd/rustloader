//! Security configuration and utilities module for Rustloader
//!
//! This module provides centralized security settings, validation functions,
//! and utilities to enhance the overall security posture of the application.

use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use ring::hmac;
use ring::rand::{SystemRandom, SecureRandom};
use ring::digest;
use base64::{Engine as _, engine::general_purpose};
use std::sync::{Once, Mutex};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::process::Command;
use std::os::unix::fs::PermissionsExt;
use regex::Regex;
use std::fs;

// Security configuration constants - moved to a central location
pub mod constants {
    use std::time::Duration;
    
    // Free version limitations
    pub const MAX_FREE_QUALITY: &str = "720";
    pub const FREE_MP3_BITRATE: &str = "128K";
    
    // Security limits
    pub const MAX_DAILY_DOWNLOADS: u32 = 5;  // Maximum daily downloads for free version
    pub const ACTIVATION_MAX_ATTEMPTS: usize = 5;  // Maximum license activation attempts
    pub const ACTIVATION_LOCKOUT_DURATION: Duration = Duration::from_secs(1800);  // 30 minutes
    pub const HASH_ITERATIONS: u32 = 10000;  // PBKDF2 iterations for key derivation
    pub const MAX_PATH_LENGTH: usize = 4096; // Maximum allowed path length
    pub const MAX_URL_LENGTH: usize = 2048;  // Maximum allowed URL length
    
    // Rate limiting
    pub const URL_VALIDATION_MAX_ATTEMPTS: usize = 20;
    pub const URL_VALIDATION_WINDOW: Duration = Duration::from_secs(60);
    pub const DOWNLOAD_MAX_ATTEMPTS: usize = 10;
    pub const DOWNLOAD_WINDOW: Duration = Duration::from_secs(60);
    
    // Temporary file permissions
    pub const SECURE_DIR_PERMISSIONS: u32 = 0o700; // Owner read/write/execute only
    pub const SECURE_FILE_PERMISSIONS: u32 = 0o600; // Owner read/write only
}

// Sensitive directory patterns to avoid in path traversal checks
pub const SENSITIVE_DIRECTORIES: [&str; 12] = [
    "/etc", "/bin", "/sbin", "/usr/bin", "/usr/sbin",
    "/usr/local/bin", "/usr/local/sbin", "/var/run",
    "/boot", "/dev", "/proc", "/sys"
];

// Additional sensitive Windows directories
#[cfg(target_os = "windows")]
pub const WINDOWS_SENSITIVE_DIRECTORIES: [&str; 5] = [
    r"C:\Windows", r"C:\Program Files", r"C:\Program Files (x86)",
    r"C:\ProgramData", r"C:\System Volume Information"
];

// Initialize the secure random number generator and rate limits using once_cell
static SECURE_RNG: Lazy<SystemRandom> = Lazy::new(|| SystemRandom::new());
static RATE_LIMITS: Lazy<Mutex<HashMap<String, Vec<Instant>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Initialization flag for security module
static INIT: Once = Once::new();

/// Initialize the security module
pub fn init() {
    INIT.call_once(|| {
        // Perform one-time security initialization
        
        // Verify integrity of security-critical files
        if let Err(e) = verify_application_integrity() {
            eprintln!("WARNING: Application integrity check failed: {}", e);
        }
        
        // Set secure process limits (where available)
        #[cfg(unix)]
        if let Err(e) = set_process_limits() {
            eprintln!("WARNING: Failed to set process limits: {}", e);
        }
        
        // Initialize secure temporary directory
        if let Err(e) = initialize_secure_temp_dir() {
            eprintln!("WARNING: Failed to initialize secure temporary directory: {}", e);
        }
    });
}

/// Generate a random secure token of specified length
pub fn generate_secure_token(length: usize) -> Result<String, AppError> {
    let mut bytes = vec![0u8; length];
    SECURE_RNG.fill(&mut bytes)
        .map_err(|_| AppError::SecurityViolation)?;
    
    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Apply rate limiting to security-sensitive operations
/// Returns true if the operation is allowed, false if rate limited
pub fn apply_rate_limit(operation: &str, max_attempts: usize, window: Duration) -> bool {
    let now = Instant::now();
    let mut limits = match RATE_LIMITS.lock() {
        Ok(guard) => guard,
        Err(_) => {
            // If we can't acquire the lock, default to allowing the operation
            // but log a warning
            eprintln!("WARNING: Could not acquire rate limit lock. Allowing operation by default.");
            return true;
        }
    };
    
    // Get or create the entry for this operation
    let attempts = limits.entry(operation.to_string()).or_insert_with(Vec::new);
    
    // Remove attempts outside the time window
    attempts.retain(|time| now.duration_since(*time) < window);
    
    // Check if we've exceeded the limit
    if attempts.len() >= max_attempts {
        return false;
    }
    
    // Add this attempt
    attempts.push(now);
    true
}

/// Generate an HMAC signature for the provided data
pub fn generate_hmac_signature(data: &[u8], key: &[u8]) -> Result<Vec<u8>, AppError> {
    let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let signature = hmac::sign(&hmac_key, data);
    Ok(signature.as_ref().to_vec())
}

/// Verify an HMAC signature for the provided data
pub fn verify_hmac_signature(data: &[u8], signature: &[u8], key: &[u8]) -> Result<bool, AppError> {
    let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    
    match hmac::verify(&hmac_key, data, signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Enhanced path safety validation with centralized security settings
pub fn validate_path_safety(path: &Path) -> Result<(), AppError> {
    // First, check path length
    let path_str = path.to_string_lossy();
    if path_str.len() > constants::MAX_PATH_LENGTH {
        return Err(AppError::ValidationError(format!(
            "Path exceeds maximum allowed length of {} characters", 
            constants::MAX_PATH_LENGTH
        )));
    }
    
    // Check for null bytes and control characters
    if path_str.contains('\0') || path_str.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
        return Err(AppError::SecurityViolation);
    }
    
    // Canonicalize the path to resolve any .. or symlinks
    // For paths that don't exist yet, we'll check components
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If path doesn't exist yet, we need to check its components
            check_path_components(path)?;
            
            // For non-existent paths, we'll use the original path for further checks
            path.to_path_buf()
        }
    };
    
    // Get user's home directory for comparison
    let home_dir = match dirs_next::home_dir() {
        Some(dir) => dir,
        None => return Err(AppError::PathError("Could not determine home directory".to_string())),
    };
    
    // Get the canonical form of the home directory
    let canonical_home = match home_dir.canonicalize() {
        Ok(h) => h,
        Err(_) => return Err(AppError::PathError("Could not canonicalize home directory".to_string())),
    };
    
    // Get system temp directory
    let temp_dir = match std::env::temp_dir().canonicalize() {
        Ok(t) => t,
        Err(_) => std::env::temp_dir(), // Fallback to non-canonicalized
    };
    
    // Get download directory (should be under home)
    let mut downloads_dir = home_dir.clone();
    downloads_dir.push("Downloads");
    
    // Convert to string for easier comparison
    let path_str = canonical_path.to_string_lossy().to_string();
    
    // Check if path is within allowed directories
    let mut allowed_paths = vec![
        canonical_home.to_string_lossy().to_string(),
        temp_dir.to_string_lossy().to_string(),
    ];
    
    // Add Downloads directory if it exists
    if downloads_dir.exists() {
        if let Ok(canon_downloads) = downloads_dir.canonicalize() {
            allowed_paths.push(canon_downloads.to_string_lossy().to_string());
        }
    }
    
    // Add data_local_dir if it exists
    if let Some(data_dir) = dirs_next::data_local_dir() {
        if let Ok(canon_data) = data_dir.canonicalize() {
            allowed_paths.push(canon_data.to_string_lossy().to_string());
        }
    }
    
    let in_allowed_path = allowed_paths.iter().any(|allowed| path_str.starts_with(allowed));
    
    if !in_allowed_path {
        return Err(AppError::SecurityViolation);
    }
    
    // Check if path contains any sensitive directories
    for dir in SENSITIVE_DIRECTORIES.iter() {
        if path_str.starts_with(dir) {
            return Err(AppError::SecurityViolation);
        }
    }
    
    // Check Windows-specific sensitive directories
    #[cfg(target_os = "windows")]
    {
        for dir in WINDOWS_SENSITIVE_DIRECTORIES.iter() {
            if path_str.to_lowercase().starts_with(&dir.to_lowercase()) {
                return Err(AppError::SecurityViolation);
            }
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
       path_str.contains("~") || path_str.contains(":") && cfg!(unix) {
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

/// Verify the integrity of security-critical files
fn verify_application_integrity() -> Result<(), AppError> {
    // Get the path to the executable
    let exe_path = std::env::current_exe()
        .map_err(|e| AppError::IoError(e))?;
    
    if !exe_path.exists() {
        return Err(AppError::SecurityViolation);
    }
    
    // In a real implementation, we would compute the hash of the executable
    // and verify it against a known good value stored securely
    // For demonstration, we'll just check if the file exists and has the right permissions
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        
        let metadata = fs::metadata(&exe_path)
            .map_err(|e| AppError::IoError(e))?;
        
        // Check if executable bit is set
        let mode = metadata.mode();
        if mode & 0o111 == 0 {
            return Err(AppError::SecurityViolation);
        }
    }
    
    Ok(())
}

/// Set secure process limits (Unix-only)
#[cfg(unix)]
fn set_process_limits() -> Result<(), AppError> {
    // Import rlimit functions from libc
    use libc::{rlimit, setrlimit, RLIMIT_NOFILE, RLIMIT_CORE, RLIMIT_CPU};
    
    unsafe {
        // Set file descriptor limit
        let mut rlim = rlimit {
            rlim_cur: 1024,
            rlim_max: 4096,
        };
        
        if setrlimit(RLIMIT_NOFILE, &rlim) != 0 {
            // Non-fatal error, just log it
            eprintln!("WARNING: Failed to set RLIMIT_NOFILE");
        }
        
        // Disable core dumps for security
        rlim.rlim_cur = 0;
        rlim.rlim_max = 0;
        
        if setrlimit(RLIMIT_CORE, &rlim) != 0 {
            eprintln!("WARNING: Failed to set RLIMIT_CORE");
        }
        
        // Limit CPU time to prevent runaway processes
        rlim.rlim_cur = 600; // 10 minutes
        rlim.rlim_max = 1200; // 20 minutes
        
        if setrlimit(RLIMIT_CPU, &rlim) != 0 {
            eprintln!("WARNING: Failed to set RLIMIT_CPU");
        }
    }
    
    Ok(())
}

#[cfg(not(unix))]
fn set_process_limits() -> Result<(), AppError> {
    // No-op for non-Unix platforms
    Ok(())
}

/// Initialize a secure temporary directory
fn initialize_secure_temp_dir() -> Result<PathBuf, AppError> {
    let temp_base = std::env::temp_dir();
    let rand_suffix = generate_secure_token(16)?;
    let temp_dir = temp_base.join(format!("rustloader_{}", rand_suffix));
    
    // Create directory with secure permissions
    fs::create_dir_all(&temp_dir)
        .map_err(|e| AppError::IoError(e))?;
    
    #[cfg(unix)]
    {
        // Set secure permissions on Unix systems
        let metadata = fs::metadata(&temp_dir)
            .map_err(|e| AppError::IoError(e))?;
        
        let mut permissions = metadata.permissions();
        permissions.set_mode(constants::SECURE_DIR_PERMISSIONS);
        
        fs::set_permissions(&temp_dir, permissions)
            .map_err(|e| AppError::IoError(e))?;
    }
    
    Ok(temp_dir)
}

/// Securely escape shell arguments to prevent command injection
pub fn escape_shell_arg(arg: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Windows cmd.exe escaping - double quotes around the string
        // and escape internal quotes with backslash
        let escaped = arg.replace("\"", "\\\"");
        format!("\"{}\"", escaped)
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Unix shell escaping - single quotes are safest
        // but need special handling for strings containing single quotes
        if !arg.contains('\'') {
            // No single quotes - just wrap in single quotes
            format!("'{}'", arg)
        } else {
            // Replace single quotes with '\'' and wrap the whole thing
            let escaped = arg.replace("'", "'\\''");
            format!("'{}'", escaped)
        }
    }
}

/// Sanitize a string to make it safe for command-line use
/// This is a defense-in-depth measure in addition to proper shell escaping
pub fn sanitize_command_arg(arg: &str) -> Result<String, AppError> {
    // First check for obviously dangerous character sequences
    let dangerous_chars = [';', '&', '|', '>', '<', '`', '$', '(', ')'];
    if arg.chars().any(|c| dangerous_chars.contains(&c)) {
        return Err(AppError::ValidationError(format!(
            "Argument contains potentially dangerous characters: {}", arg
        )));
    }
    
    // Define specific allowlists for different argument types
    
    // For bitrate arguments (e.g., 1000K)
    if arg.ends_with('K') || arg.ends_with('M') {
        let num_part = &arg[0..arg.len()-1];
        if num_part.chars().all(|c| c.is_ascii_digit()) {
            return Ok(arg.to_string());
        }
    }
    
    // For time arguments (e.g., 00:01:30)
    if arg.len() == 8 && arg.chars().nth(2) == Some(':') && arg.chars().nth(5) == Some(':') {
        let time_parts: Vec<&str> = arg.split(':').collect();
        if time_parts.len() == 3 && 
           time_parts.iter().all(|part| part.chars().all(|c| c.is_ascii_digit())) {
            return Ok(arg.to_string());
        }
    }
    
    // For format arguments (mp3, mp4, etc.)
    if ["mp3", "mp4", "webm", "m4a", "flac", "wav", "ogg"].contains(&arg) {
        return Ok(arg.to_string());
    }
    
    // For quality specifiers
    if ["480", "720", "1080", "2160", "best", "bestaudio"].contains(&arg) ||
       (arg.starts_with("best[") && arg.ends_with("]")) {
        return Ok(arg.to_string());
    }
    
    // For URLs - apply URL validation
    if arg.starts_with("http://") || arg.starts_with("https://") {
        validate_url(arg)?;
        return Ok(arg.to_string());
    }
    
    // For paths - validate separately
    if arg.contains('/') || arg.contains('\\') {
        let path = std::path::Path::new(arg);
        validate_path_safety(path)?;
        return Ok(arg.to_string());
    }
    
    // General whitelist for other arguments
    let valid_chars = arg.chars().all(|c| 
        c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-' || 
        c == '.' || c == ':' || c == '=' || c == '[' || c == ']'
    );
    
    if !valid_chars {
        return Err(AppError::ValidationError(format!("Invalid characters in argument: {}", arg)));
    }
    
    Ok(arg.to_string())
}

/// Check for potential command injection patterns
pub fn detect_command_injection(input: &str) -> bool {
    // Look for shell metacharacters and other dangerous patterns
    let suspicious_patterns = [
        ";", "&", "&&", "||", "|", "`", "$(",
        "$()", ">${", ">%", "<${", "<%", "}}%", "$[", "\\x",
        "eval", "exec", "system", "os.system", "Process", "popen",
        "fork", "\\n", "\\r", "\\t", "\\b", "\\f", "\\v"
    ];
    
    for pattern in suspicious_patterns.iter() {
        if input.contains(pattern) {
            return true; // Suspicious pattern found
        }
    }
    
    // Check for attempted escaping of quotes
    if input.contains("\\\"") || input.contains("\\'") {
        return true;
    }
    
    // Check for environment variable access
    if input.contains("$") && (input.contains("{") || input.contains("(")) {
        return true;
    }
    
    // Check for hex/octal/unicode escape sequences
    let escape_regex = Regex::new(r"\\[xXuU][0-9a-fA-F]").unwrap();
    if escape_regex.is_match(input) {
        return true;
    }
    
    false // No suspicious patterns found
}

/// Validate URL format with enhanced security checks
pub fn validate_url(url: &str) -> Result<(), AppError> {
    // Apply rate limiting to URL validation to prevent DoS
    if !apply_rate_limit("url_validation", 
                          constants::URL_VALIDATION_MAX_ATTEMPTS, 
                          constants::URL_VALIDATION_WINDOW) {
        return Err(AppError::ValidationError(
            "Too many validation attempts. Please try again later.".to_string()
        ));
    }
    
    // Check URL length
    if url.len() > constants::MAX_URL_LENGTH {
        return Err(AppError::ValidationError(
            format!("URL exceeds maximum allowed length of {} characters", 
                   constants::MAX_URL_LENGTH)
        ));
    }
    
    // Basic URL validation - use regex for basic structure
    let url_regex = Regex::new(r"^https?://(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?(?:/[^\s]*)?$").unwrap();
    
    // Allow common video platform URLs with specific patterns
    let youtube_regex = Regex::new(r"^https?://(?:www\.)?(?:youtube\.com|youtu\.be)/").unwrap();
    let vimeo_regex = Regex::new(r"^https?://(?:www\.)?vimeo\.com/").unwrap();
    let dailymotion_regex = Regex::new(r"^https?://(?:www\.)?dailymotion\.com/").unwrap();
    
    if !(url_regex.is_match(url) || youtube_regex.is_match(url) || 
         vimeo_regex.is_match(url) || dailymotion_regex.is_match(url)) {
        return Err(AppError::ValidationError(
            format!("Invalid URL format: {}", url)
        ));
    }
    
    // Check for command injection
    if detect_command_injection(url) {
        return Err(AppError::SecurityViolation);
    }
    
    // Prohibit non-standard URL protocols and ports
    if url.contains("://") && !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::ValidationError(
            "Only HTTP and HTTPS protocols are supported".to_string()
        ));
    }
    
    // Validate URL does not target internal network
    let localhost_patterns = [
        "localhost", "127.", "10.", "192.168.", "172.16.", "172.17.", 
        "172.18.", "172.19.", "172.20.", "172.21.", "172.22.", "172.23.", 
        "172.24.", "172.25.", "172.26.", "172.27.", "172.28.", "172.29.", 
        "172.30.", "172.31.", "169.254.", "::1", "0.0.0.0", "0:0:0:0:0:0:0:0"
    ];
    
    for pattern in &localhost_patterns {
        // Extract hostname from URL
        if let Some(host_start) = url.find("://") {
            let after_proto = &url[host_start + 3..];
            let host_end = after_proto.find('/').unwrap_or(after_proto.len());
            let hostname = &after_proto[..host_end];
            
            // Check if hostname contains any forbidden pattern
            if hostname.contains(pattern) {
                return Err(AppError::ValidationError(
                    "URLs targeting internal networks are not allowed".to_string()
                ));
            }
        }
    }
    
    Ok(())
}

/// Verify file integrity using a cryptographic hash
pub fn verify_file_integrity(file_path: &Path, expected_hash: &str) -> Result<bool, AppError> {
    use std::fs::File;
    use std::io::Read;
    
    // Open the file
    let mut file = File::open(file_path).map_err(|e| AppError::IoError(e))?;
    
    // Create a new SHA-256 digest context
    let mut context = digest::Context::new(&digest::SHA256);
    let mut buffer = [0; 8192];
    
    // Read the file in chunks and update the digest
    loop {
        let count = file.read(&mut buffer).map_err(|e| AppError::IoError(e))?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    
    // Finalize the digest and encode as base64
    let digest = context.finish();
    let hash = general_purpose::STANDARD_NO_PAD.encode(digest.as_ref());
    
    // Compare with expected hash
    Ok(hash == expected_hash)
}

/// Secure deletion of sensitive files
pub fn secure_delete_file(file_path: &Path) -> Result<(), AppError> {
    use std::fs::{OpenOptions, remove_file};
    use std::io::{Write, Seek, SeekFrom};
    
    // Open the file for writing
    let mut file = OpenOptions::new()
        .write(true)
        .open(file_path)
        .map_err(|e| AppError::IoError(e))?;
    
    // Get file size
    let file_size = file.metadata()
        .map_err(|e| AppError::IoError(e))?
        .len() as usize;
    
    // Generate random data buffer
    let mut buffer = vec![0u8; std::cmp::min(8192, file_size)];
    
    // Overwrite file with random data three times for better security
    for _ in 0..3 {
        // Seek to beginning of file
        file.seek(SeekFrom::Start(0))
            .map_err(|e| AppError::IoError(e))?;
        
        // Fill file with random data
        let mut remaining = file_size;
        while remaining > 0 {
            let chunk_size = std::cmp::min(buffer.len(), remaining);
            SECURE_RNG.fill(&mut buffer[..chunk_size])
                .map_err(|_| AppError::SecurityViolation)?;
            
            file.write_all(&buffer[..chunk_size])
                .map_err(|e| AppError::IoError(e))?;
            
            remaining -= chunk_size;
        }
        
        // Flush to ensure write
        file.flush().map_err(|e| AppError::IoError(e))?;
    }
    
    // Close the file
    drop(file);
    
    // Remove the file
    remove_file(file_path).map_err(|e| AppError::IoError(e))?;
    
    Ok(())
}

/// Execute a command safely without shell interpretation
pub fn safe_execute_command(program: &str, args: &[&str]) -> Result<String, AppError> {
    // Validate the program name for basic safety
    if program.contains('/') || program.contains('\\') {
        return Err(AppError::SecurityViolation);
    }
    
    // Validate each argument
    for arg in args {
        // Just check for null bytes as a basic safety measure
        if arg.contains('\0') {
            return Err(AppError::SecurityViolation);
        }
        
        // For more comprehensive validation, the safest approach is to
        // use an allowlist of valid characters per argument type,
        // but that's outside the scope of this function
    }
    
    // Execute the command directly without shell interpretation
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| AppError::IoError(e))?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(AppError::General(format!(
            "Command {} failed: {}", 
            program, 
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

/// Create a secure temporary file with proper permissions
pub fn create_secure_temp_file(prefix: &str, suffix: &str) -> Result<(PathBuf, fs::File), AppError> {
    let temp_dir = std::env::temp_dir();
    let rand_part = generate_secure_token(16)?;
    let file_name = format!("{}_{}{}", prefix, rand_part, suffix);
    let file_path = temp_dir.join(file_name);
    
    // Create the file
    let file = fs::File::create(&file_path)
        .map_err(|e| AppError::IoError(e))?;
    
    #[cfg(unix)]
    {
        // Set secure permissions on Unix systems
        let metadata = file.metadata()
            .map_err(|e| AppError::IoError(e))?;
        
        let mut permissions = metadata.permissions();
        permissions.set_mode(constants::SECURE_FILE_PERMISSIONS);
        
        fs::set_permissions(&file_path, permissions)
            .map_err(|e| AppError::IoError(e))?;
    }
    
    Ok((file_path, file))
}

/// Create a secure temporary directory with proper permissions
pub fn create_secure_temp_dir(prefix: &str) -> Result<PathBuf, AppError> {
    let temp_dir = std::env::temp_dir();
    let rand_part = generate_secure_token(16)?;
    let dir_name = format!("{}_{}", prefix, rand_part);
    let dir_path = temp_dir.join(dir_name);
    
    // Create the directory
    fs::create_dir_all(&dir_path)
        .map_err(|e| AppError::IoError(e))?;
    
    #[cfg(unix)]
    {
        // Set secure permissions on Unix systems
        let metadata = fs::metadata(&dir_path)
            .map_err(|e| AppError::IoError(e))?;
        
        let mut permissions = metadata.permissions();
        permissions.set_mode(constants::SECURE_DIR_PERMISSIONS);
        
        fs::set_permissions(&dir_path, permissions)
            .map_err(|e| AppError::IoError(e))?;
    }
    
    Ok(dir_path)
}