//! GUI module
#![allow(unused_imports)]

pub mod app;
pub mod views;
pub mod components;
pub mod integration;
pub mod clipboard;
pub mod theme;

// Re-export for convenience
pub use app::RustloaderApp;
pub use app::Message;
pub use app::View;
