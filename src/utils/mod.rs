//! Utility modules for error handling and configuration
#![allow(dead_code, unused_variables, unused_imports)]

pub mod bundle_paths;
pub mod config;
pub mod error;
pub mod metadata;
pub mod organizer;
pub mod platform;

// Re-export for convenience
pub use bundle_paths::{
    get_app_support_dir, get_database_path, get_download_file_path, get_downloads_dir,
};
pub use config::AppSettings;
pub use error::RustloaderError;
pub use metadata::{MetadataManager, MetadataStats, VideoMetadata};
pub use organizer::{ContentType, FileOrganizer, OrganizationSettings, OrganizeMode, QualityTier};
