use std::io;
use thiserror::Error;

/// Custom error types for the application
#[derive(Error, Debug)]
pub enum AppError {
    /// Error for missing dependencies
    #[error("Missing dependency: {0}")]
    MissingDependency(String),
    
    /// Error during download process
    #[error("Download error: {0}")]
    DownloadError(String),
    
    /// Error for invalid input validation
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    /// I/O related errors
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    
    /// Error for invalid time format
    #[error("Time format error: {0}")]
    TimeFormatError(String),
    
    /// Error for path operation failures
    #[error("Path error: {0}")]
    PathError(String),
    
    /// General application errors
    #[error("Application error: {0}")]
    General(String),
}

/// Convert a string error to AppError::General
impl From<String> for AppError {
    fn from(error: String) -> Self {
        AppError::General(error)
    }
}

/// Convert a &str error to AppError::General
impl From<&str> for AppError {
    fn from(error: &str) -> Self {
        AppError::General(error.to_string())
    }
}