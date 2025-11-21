//! Database module

pub mod schema;
pub mod operations;

// Re-export for convenience
pub use schema::initialize_database;
pub use operations::{DatabaseManager, DownloadRecord, SettingsRecord};
