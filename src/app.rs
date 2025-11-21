//! Application initialization and main entry point

use anyhow::Result;
use gui::RustloaderApp;
use iced::{Application, Settings};

/// Run the Rustloader application
pub async fn run() -> Result<()> {
    // Initialize the application
    let mut settings = Settings::default();
    settings.window.size = (800, 600);
    settings.window.min_size = Some((600, 400));

    // Run the GUI
    RustloaderApp::run(settings)?;

    Ok(())
}
