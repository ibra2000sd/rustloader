//! Application initialization and main entry point

use anyhow::Result;
use crate::gui::RustloaderApp;
use iced::{Application, Settings};
use iced::Size;

/// Run the Rustloader application
pub async fn run() -> Result<()> {
    // Initialize the application
    let mut settings = Settings::default();
    settings.window.size = Size::new(800.0, 600.0);
    settings.window.min_size = Some(Size::new(600.0, 400.0));

    // Run the GUI
    RustloaderApp::run(settings)?;

    Ok(())
}
