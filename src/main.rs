//! Rustloader - High-Performance Video Downloader
//!
//! A cross-platform video downloader that combines yt-dlp extraction capabilities
//! with a fast Rust-based download engine and a simple, practical GUI.

use anyhow::Result;
use clap::Parser;
use iced::Application;
use rustloader::cli::Cli;
use rustloader::gui;
use std::process::Command;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Check for yt-dlp
    check_ytdlp_installed();

    if cli.is_cli_mode() {
        // Run a headless download through the existing engine inside a
        // temporary Tokio runtime, then exit (no GUI).
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async move { rustloader::cli::run(&cli).await })?;
        return Ok(());
    }

    // Start the GUI application (synchronous entrypoint)
    gui::RustloaderApp::run(iced::Settings {
        window: iced::window::Settings {
            size: iced::Size::new(900.0, 600.0),
            min_size: Some(iced::Size::new(800.0, 500.0)),
            decorations: true, // Keep decorations for now as custom title bars are complex in Iced without winit direct access
            transparent: false, // Transparency can be tricky across platforms
            icon: gui::icon::load_icon(), // Load application icon
            ..Default::default()
        },
        antialiasing: true,
        ..Default::default()
    })?;

    Ok(())
}

fn check_ytdlp_installed() {
    // Try common macOS yt-dlp installation paths when launched from Finder/Dock
    // (where PATH may not include user-installed Python binaries)
    let possible_paths = [
        "yt-dlp", // Try PATH first
        "/usr/local/bin/yt-dlp",
        "/opt/homebrew/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.12/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.11/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.10/bin/yt-dlp",
    ];

    for path in &possible_paths {
        if let Ok(output) = Command::new(path).arg("--version").output() {
            if output.status.success() {
                println!("✓ yt-dlp found at: {}", path);
                return;
            }
        }
    }

    // yt-dlp not found - warn but don't exit, allow app to launch
    // User will see error when they try to add a URL
    eprintln!("WARNING: yt-dlp not found in common locations");
    eprintln!("The app will run, but video extraction will fail.");
    eprintln!("Please install yt-dlp:");
    eprintln!("  pip install yt-dlp");
    eprintln!("  or: brew install yt-dlp");
    eprintln!("  or visit: https://github.com/yt-dlp/yt-dlp");
}
