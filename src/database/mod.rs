//! Database module

pub mod operations;
pub mod schema;

// Re-export for convenience
#[allow(unused_imports)] // Exposed for external callers; may be unused internally
pub use operations::{DatabaseManager, DownloadRecord, SettingsRecord};
pub use schema::initialize_database;
