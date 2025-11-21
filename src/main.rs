//! Rustloader - High-Performance Video Downloader
//! 
//! A cross-platform video downloader that combines yt-dlp extraction capabilities
//! with a fast Rust-based download engine and a simple, practical GUI.

mod app;
mod extractor;
mod downloader;
mod queue;
mod database;
mod gui;
mod utils;

use anyhow::Result;
use clap::Parser;
use std::process::Command;
use tracing_subscriber;

#[derive(Parser)]
struct Args {
    /// Test download with provided URL
    #[arg(long)]
    test_download: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Check for yt-dlp
    check_ytdlp_installed();
    
    if let Some(url) = args.test_download {
        // Run headless test
        test_download_cli(url).await;
        return Ok(());
    }
    
    // Start the GUI application
    gui::RustloaderApp::run(iced::Settings {
        window: iced::window::Settings {
            size: iced::Size::new(800.0, 600.0),
            min_size: Some(iced::Size::new(600.0, 400.0)),
            ..Default::default()
        },
        ..Default::default()
    })?;
    
    Ok(())
}

fn check_ytdlp_installed() {
    let check = Command::new("yt-dlp")
        .arg("--version")
        .output();
        
    match check {
        Ok(output) if output.status.success() => {
            println!("✓ yt-dlp found");
        }
        _ => {
            eprintln!("ERROR: yt-dlp not found!");
            eprintln!("Please install yt-dlp:");
            eprintln!("  pip install yt-dlp");
            eprintln!("  or visit: https://github.com/yt-dlp/yt-dlp");
            std::process::exit(1);
        }
    }
}

async fn test_download_cli(url: String) {
    println!("Testing download: {}", url);
    
    // Initialize extractor
    let extractor = match extractor::VideoExtractor::new() {
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
    
    // Create output path
    let output_path = std::path::PathBuf::from("./test_download.mp4");
    
    // Create progress channel
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(100);
    
    // Spawn progress reporter
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            println!("Progress: {:.1}%, Speed: {:.2} MB/s", 
                progress.percentage() * 100.0, 
                progress.speed / 1024.0 / 1024.0);
        }
    });
    
    // Start download
    println!("Starting download...");
    match engine.download(&video_info.direct_url, &output_path, progress_tx).await {
        Ok(_) => println!("Download completed successfully!"),
        Err(e) => eprintln!("Download failed: {}", e),
    }
}
