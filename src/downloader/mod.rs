//! Download engine module

pub mod engine;
pub mod merger;
pub mod progress;
pub mod segment;

// Re-export for convenience
#[allow(unused_imports)] // Exposed for external callers; may be unused internally
pub use engine::{DownloadConfig, DownloadEngine};
#[allow(unused_imports)] // Exposed for external callers; may be unused internally
pub use progress::{DownloadProgress, DownloadStatus};
