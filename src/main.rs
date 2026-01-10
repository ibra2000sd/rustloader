//! Rustloader - High-Performance Video Downloader
//!
//! A cross-platform video downloader that combines yt-dlp extraction capabilities
//! with a fast Rust-based download engine and a simple, practical GUI.

use anyhow::Result;
use clap::Parser;
use iced::Application;
use rustloader::downloader;
use rustloader::extractor::{self, Extractor};
use rustloader::gui;
use rustloader::utils;
use std::process::Command;

#[derive(Parser)]
struct Args {
    /// Test download with provided URL
    #[arg(long)]
    test_download: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Check for yt-dlp
    check_ytdlp_installed();

    if let Some(url) = args.test_download {
        // Run headless test inside a temporary Tokio runtime
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async move {
            test_download_cli(url).await;
        });
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

async fn test_download_cli(url: String) {
    println!("Testing download: {}", url);

    // Initialize extractor
    let extractor = match extractor::YtDlpExtractor::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to initialize extractor: {}", e);
            return;
        }
    };

    // Extract video info
    println!("Extracting video info...");
    let video_info = match extractor.extract_info(&url).await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Failed to extract video info: {}", e);
            return;
        }
    };

    println!("Title: {}", video_info.title);
    println!("Duration: {:?}", video_info.duration);
    println!("File size: {:?}", video_info.filesize);

    // Initialize download engine
    let config = downloader::DownloadConfig::default();
    let engine = downloader::DownloadEngine::new(config);

    // Create output path using bundle-aware utilities
    // Use ~/Downloads/test_download.mp4 instead of relative path ./test_download.mp4
    // to ensure the file is saved to the correct location even when launched from Finder/Dock
    let output_path = utils::get_download_file_path("test_download.mp4");

    // Create progress channel
    let (progress_tx, mut progress_rx) =
        tokio::sync::mpsc::channel::<crate::downloader::DownloadProgress>(100);

    // Spawn progress reporter
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            println!(
                "Progress: {:.1}%, Speed: {:.2} MB/s",
                progress.percentage() * 100.0,
                progress.speed / 1024.0 / 1024.0
            );
        }
    });

    // Ensure we have a direct URL to download. If extractor didn't populate `direct_url`,
    // try to resolve one via extractor.get_direct_url using the first available format.
    println!("Starting download...");
    let download_url = if !video_info.direct_url.is_empty() {
        video_info.direct_url.clone()
    } else if let Some(first_fmt) = video_info.formats.first() {
        match extractor
            .get_direct_url(&video_info.url, &first_fmt.format_id)
            .await
        {
            Ok(u) => u,
            Err(e) => {
                eprintln!("Failed to resolve direct url via extractor: {}", e);
                video_info.url.clone()
            }
        }
    } else {
        video_info.url.clone()
    };

    match engine
        .download(&download_url, &output_path, None, progress_tx)
        .await
    {
        Ok(_) => println!("Download completed successfully!"),
        Err(e) => eprintln!("Download failed: {}", e),
    }
}
