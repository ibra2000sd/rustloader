//! GUI components

pub mod download_card;
pub mod format_selector; // Restored
pub use download_card::{DownloadCard, DownloadCardMessage, DownloadStatus};
pub use format_selector::{FormatSelector, FormatSelectorMessage};
