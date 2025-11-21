//! GUI module

pub mod app;
pub mod views;
pub mod components;
pub mod integration;
pub mod clipboard;

// Re-export for convenience
pub use app::RustloaderApp;
pub use app::Message;
pub use app::View;
