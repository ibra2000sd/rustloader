//! GUI module
#![allow(unused_imports)]

pub mod app;
pub mod clipboard;
pub mod components;
pub mod icon;
pub mod integration;
pub mod theme;
pub mod views;

// Re-export for convenience
pub use app::Message;
pub use app::RustloaderApp;
pub use app::View;
