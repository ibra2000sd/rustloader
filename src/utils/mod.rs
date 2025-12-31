//! Utility modules for error handling and configuration
#![allow(dead_code, unused_variables, unused_imports)]

pub mod config;
pub mod error;
pub mod metadata;
pub mod organizer;

// Re-export for convenience
pub use config::AppSettings;
pub use error::RustloaderError;
pub use metadata::{MetadataManager, MetadataStats, VideoMetadata};
pub use organizer::{ContentType, FileOrganizer, OrganizationSettings, OrganizeMode, QualityTier};
