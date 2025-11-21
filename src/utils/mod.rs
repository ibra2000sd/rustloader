//! Utility modules for error handling and configuration

pub mod config;
pub mod error;

// Re-export for convenience
pub use config::AppSettings;
pub use error::RustloaderError;
