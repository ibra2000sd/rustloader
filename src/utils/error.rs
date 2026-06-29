//! Error handling for Rustloader

use thiserror::Error;

/// Main error type for Rustloader
#[derive(Debug, Error)]
pub enum RustloaderError {
    #[error("yt-dlp not found. Please install yt-dlp")]
    YtDlpNotFound,

    #[error("Failed to extract video info: {0}")]
    ExtractionError(String),

    #[error("Download failed: {0}")]
    DownloadError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Map a raw error string (from yt-dlp / reqwest / the engine) to a concise,
/// user-friendly message. Shared by the GUI and the CLI so both surface the
/// same human-readable text instead of raw internals.
pub fn make_error_user_friendly(error: &str) -> String {
    let error_lower = error.to_lowercase();

    if error_lower.contains("truncated") || error_lower.contains("incomplete") {
        "Please enter a complete and valid URL".to_string()
    } else if error_lower.contains("invalid url") || error_lower.contains("malformed") {
        "This doesn't appear to be a valid video URL".to_string()
    } else if error_lower.contains("network")
        || error_lower.contains("connection")
        || error_lower.contains("timeout")
        || error_lower.contains("dns")
        || error_lower.contains("resolve")
    {
        "Unable to connect. Please check your internet connection".to_string()
    } else if error_lower.contains("unavailable")
        || error_lower.contains("not found")
        || error_lower.contains("removed")
    {
        "This video is not available or has been removed".to_string()
    } else if error_lower.contains("private") || error_lower.contains("restricted") {
        "This video is private or restricted".to_string()
    } else if error_lower.contains("age") && error_lower.contains("restricted") {
        "This video is age-restricted and cannot be downloaded".to_string()
    } else if error_lower.contains("geo") || error_lower.contains("region") {
        "This video is not available in your region".to_string()
    } else if error_lower.contains("copyright") {
        "This video cannot be downloaded due to copyright restrictions".to_string()
    } else if error_lower.contains("403") || error_lower.contains("forbidden") {
        "Download was blocked (HTTP 403). Try updating yt-dlp and installing a JS runtime (deno)"
            .to_string()
    } else {
        "Unable to process this URL. Please try a different video".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_maps_known_errors() {
        assert!(make_error_user_friendly("dns error: failed to resolve")
            .contains("check your internet"));
        assert!(make_error_user_friendly("HTTP Error 403: Forbidden").contains("403"));
        assert!(make_error_user_friendly("video is private").contains("private"));
        assert_eq!(
            make_error_user_friendly("some totally unexpected thing"),
            "Unable to process this URL. Please try a different video"
        );
    }
}
