// src/main.rs

mod cli;
mod downloader;
mod dependency_validator;
mod error;
mod security;
mod utils;
mod license;
mod ytdlp_wrapper;
mod ffmpeg_wrapper;
mod counter;
mod promo;

use cli::build_cli;
use colored::*;
use dependency_validator::validate_dependencies;
use downloader::{download_video_free, download_video_pro};
use error::AppError;
use utils::check_for_updates;
use license::{is_pro_version, display_license_info, activate_license, LicenseStatus};
use rand::Rng;

// Logo and version information
const VERSION: &str = "1.0.0";

// Only include the startup messages for main.rs since that's all we use here
struct StartupPromo {
    messages: Vec<String>,
}

impl StartupPromo {
    fn new() -> Self {
        Self {
            messages: vec![
                "🚀 Rustloader Pro offers 4K video downloads and 5X faster speeds! 🚀".to_string(),
                "💎 Upgrade to Rustloader Pro for AI-powered video upscaling! 💎".to_string(),
                "🎵 Enjoy high-quality 320kbps MP3 and FLAC with Rustloader Pro! 🎵".to_string(),
                "🔥 Rustloader Pro removes daily download limits! 🔥".to_string(),
            ],
        }
    }
    
    fn get_random_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.messages.len());
        &self.messages[idx]
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Set up error handling for the whole program
    let result = run_main().await;
    
    match &result {
        Ok(_) => {
            println!("{}", "Process completed successfully.".green());
        },
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            
            // Provide more helpful error messages for common errors
            match e {
                AppError::MissingDependency(msg) => {
                    eprintln!("{}: {}", "Solution".green(), msg);
                    if msg.contains("yt-dlp") {
                        eprintln!("You can install yt-dlp by running: pip install yt-dlp");
                    } else if msg.contains("ffmpeg") {
                        if cfg!(target_os = "windows") {
                            eprintln!("You can download ffmpeg from: https://ffmpeg.org/download.html");
                        } else if cfg!(target_os = "macos") {
                            eprintln!("You can install ffmpeg by running: brew install ffmpeg");
                        } else {
                            eprintln!("You can install ffmpeg by running: sudo apt install ffmpeg (or your distro's equivalent)");
                        }
                    }
                },
                AppError::DailyLimitExceeded => {
                    eprintln!("{}", "Daily download limit exceeded for free version.".red().bold());
                    eprintln!("{}", "🚀 Upgrade to Rustloader Pro for unlimited downloads: rustloader.com/pro 🚀".bright_yellow());
                    eprintln!("\nAlternatively, you can bypass the limit for testing by setting this environment variable:");
                    eprintln!("export RUSTLOADER_BYPASS_LIMIT=1");
                },
                AppError::PathError(msg) => {
                    eprintln!("{}: Check that you have correct permissions for the path", "Solution".green());
                    eprintln!("Also check if the directory exists and is accessible");
                    eprintln!("\nAlternatively, specify a custom output directory with the -o option:");
                    eprintln!("rustloader URL -o /path/to/downloads");
                },
                AppError::DownloadError(msg) => {
                    if msg.contains("403") {
                        eprintln!("{}: The website may be blocking downloads", "Solution".green());
                        eprintln!("Try updating yt-dlp to the latest version: pip install -U yt-dlp");
                    } else if msg.contains("not found") {
                        eprintln!("{}: Check that the URL is correct and the video exists", "Solution".green());
                    } else {
                        eprintln!("{}: Check your internet connection and try again", "Solution".green());
                    }
                },
                _ => {
                    eprintln!("For more help, visit: rustloader.com/help");
                }
            }
        }
    }
    
    result
}

// Separated main logic for better error handling
async fn run_main() -> Result<(), AppError> {
    // Display logo and welcome message
    print_logo();
    
    // Check for updates in the background
    let update_check = tokio::spawn(check_for_updates());
    
    // Check license status
    let is_pro = is_pro_version();
    
    if is_pro {
        println!("{}", "Rustloader Pro - Advanced Video Downloader".bright_cyan().bold());
        // Display license information if in Pro mode
        if let Err(e) = display_license_info() {
            eprintln!("{}: {}", "Warning".yellow(), e);
        }
    } else {
        println!("{}", "Rustloader - Video Downloader".bright_cyan().bold());
        println!("{}", format!("Version: {} (Free)", VERSION).cyan());
        
        // Display a promotional message for the free version
        let promo = StartupPromo::new();
        println!("\n{}\n", promo.get_random_message().bright_yellow());
    }
    
    // Initialize native FFmpeg library
    if let Err(e) = ffmpeg_wrapper::init() {
        println!("{}: {}", "Warning".yellow(), e);
        println!("Will attempt to continue, but video processing may fail");
    }
    
    // Validate dependencies with more lenient error handling
    match validate_dependencies() {
        Ok(_) => {
            println!("{}", "All dependencies validated successfully.".green());
        },
        Err(e) => {
            println!("{}: {}", "Dependency validation warning".yellow(), e);
            println!("Continue anyway? (y/n):");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                return Err(e);
            } else {
                println!("{}", "Continuing with potential dependency issues...".yellow());
            }
        }
    }

    // Parse command-line arguments
    let matches = build_cli().get_matches();
    
    // Check for license activation command
    if let Some(key) = matches.get_one::<String>("activate-license") {
        println!("{}", "License activation process started...".blue());
        
        // Get email for activation
        println!("Please enter your email address:");
        let mut email = String::new();
        std::io::stdin().read_line(&mut email)?;
        email = email.trim().to_string();
        
        // Try to activate the license with better error handling
        match activate_license(key, &email) {
            Ok(LicenseStatus::Pro(license)) => {
                println!("{}", "License activated successfully!".green());
                println!("Thank you for upgrading to Rustloader Pro!");
                println!("Email: {}", license.user_email);
                println!("Activated: {}", license.activation_date);
                if let Some(exp) = license.expiration_date {
                    println!("Expires: {}", exp);
                } else {
                    println!("License Type: Perpetual (No Expiration)");
                }
                
                println!("\nPlease restart Rustloader to use Pro features.");
                return Ok(());
            },
            Ok(LicenseStatus::Invalid(reason)) => {
                println!("{}: {}", "License activation failed".red(), reason);
                return Err(AppError::LicenseError(format!("License activation failed: {}", reason)));
            },
            Ok(LicenseStatus::Free) => {
                println!("{}", "License activation failed with an unknown error".red());
                return Err(AppError::LicenseError("License activation failed".to_string()));
            },
            Err(e) => {
                println!("{}: {}", "License activation error".red(), e);
                return Err(e);
            }
        }
    }
    
    // Show license information if requested
    if matches.get_flag("license-info") {
        return display_license_info();
    }
    
    // Extract URL and options
    let url = match matches.get_one::<String>("url") {
        Some(url) => url,
        None => {
            // This shouldn't happen due to clap's required_unless_present_any,
            // but we handle it anyway for robustness
            println!("No URL provided. Use rustloader --help for usage information.");
            return Err(AppError::ValidationError("No URL provided".to_string()));
        }
    };
    
    let quality = matches.get_one::<String>("quality").map(|q| q.as_str());
    let format = matches.get_one::<String>("format").map(|f| f.as_str()).unwrap_or("mp4");
    let start_time = matches.get_one::<String>("start-time").map(|s| s.as_str());
    let end_time = matches.get_one::<String>("end-time").map(|s| s.as_str());
    let use_playlist = matches.get_flag("playlist");
    let download_subtitles = matches.get_flag("subtitles");
    let output_dir = matches.get_one::<String>("output-dir").map(|s| s.as_str());
    
    // Only allow force download in development mode with warning
    let force_download = if cfg!(debug_assertions) {
        let is_forced = matches.get_flag("force");
        if is_forced {
            println!("{}", "⚠️ WARNING: Development mode force flag enabled! Daily limits bypassed. ⚠️".bright_red());
            println!("{}", "This flag should never be used in production environments.".bright_red());
        }
        is_forced
    } else {
        false
    };
    
    let bitrate = matches.get_one::<String>("video-bitrate").map(|b| b.as_str());

    // Check for update results
    if let Ok(update_result) = update_check.await {
        if let Ok(true) = update_result {
            println!("{}", "A new version of Rustloader is available! Visit rustloader.com to upgrade.".bright_yellow());
        }
    }

    // Use different download function based on license status
    if is_pro {
        // For Pro users, use the enhanced Pro download function
        download_video_pro::<fn(u64, u64) -> bool>(
            url, 
            quality, 
            format, 
            start_time, 
            end_time, 
            use_playlist,
            download_subtitles,
            output_dir,
            force_download,
            bitrate,
            None,  // No progress callback for now
        ).await?;
        
        println!("{}", "Pro download process completed successfully.".green());
    } else {
        // For Free users, use the standard function
        download_video_free::<fn(u64, u64) -> bool>(
            url, 
            quality, 
            format, 
            start_time, 
            end_time, 
            use_playlist,
            download_subtitles,
            output_dir,
            force_download,
            bitrate,
            None,  // No progress callback for now
        ).await?;
        
        println!("{}", "Download process completed successfully.".green());
    }

    Ok(())
}

fn print_logo() {
    println!("\n{}", r"
 ____           _   _                 _           
|  _ \ _   _ __| |_| | ___   __ _  __| | ___ _ __ 
| |_) | | | / _` | | |/ _ \ / _` |/ _` |/ _ \ '__|
|  _ <| |_| \__ | |_| | (_) | (_| | (_| |  __/ |   
|_| \_\\__,_|___/\__|_|\___/ \__,_|\__,_|\___|_|   
                                                  
".bright_cyan());
}