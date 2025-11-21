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
