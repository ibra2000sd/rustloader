//! Download engine module

pub mod engine;
pub mod merger;
pub mod progress;
pub mod resume_guard;
pub mod segment;

// Re-export for convenience
#[allow(unused_imports)] // Exposed for external callers; may be unused internally
pub use engine::{
    build_ytdlp_args, ytdlp_output_template, DownloadConfig, DownloadEngine, YtDlpOptions,
};
#[allow(unused_imports)] // Exposed for external callers; may be unused internally
pub use progress::{DownloadProgress, DownloadStatus};
