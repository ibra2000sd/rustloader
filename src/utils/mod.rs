//! Utility modules for error handling and configuration

pub mod config;
pub mod error;
pub mod organizer;
pub mod metadata;

// Re-export for convenience
pub use config::AppSettings;
pub use error::RustloaderError;
pub use organizer::{FileOrganizer, OrganizationSettings, OrganizeMode, ContentType, QualityTier};
pub use metadata::{VideoMetadata, MetadataManager, MetadataStats};
