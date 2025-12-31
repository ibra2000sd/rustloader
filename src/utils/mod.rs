//! Utility modules for error handling and configuration
#![allow(dead_code, unused_variables, unused_imports)]

pub mod config;
pub mod error;
pub mod organizer;
pub mod metadata;

// Re-export for convenience
pub use config::AppSettings;
pub use error::RustloaderError;
pub use organizer::{FileOrganizer, OrganizationSettings, OrganizeMode, ContentType, QualityTier};
pub use metadata::{VideoMetadata, MetadataManager, MetadataStats};
