use crate::error::AppError;
use crate::utils::{format_output_path, initialize_download_dir, validate_time_format, validate_url};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use notify_rust::Notification;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;

/// Download a video or audio file from the specified URL
pub async fn download_video(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&String>,
    end_time: Option<&String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<&String>,
) -> Result<(), AppError> {
    // Validate URL
    validate_url(url)?;
    
    println!("{}: {}", "Download URL".blue(), url);
    
    // Validate time formats if provided
    if let Some(start) = start_time {
        validate_time_format(start)?;
    }
    
    if let Some(end) = end_time {
        validate_time_format(end)?;
    }
    
    // Initialize download directory
    let folder_type = if format == "mp3" { "audio" } else { "videos" };
    let download_dir = initialize_download_dir(
        output_dir.map(|s| s.as_str()), 
        "rustloader", 
        folder_type
    )?;
    
    // Create the output path format
    let output_path = format_output_path(&download_dir, format)?;
    
    // Create progress bar and wrap in Arc for sharing between threads
    let pb = Arc::new(ProgressBar::new(100));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% (ETA: {eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Build yt-dlp command
    let mut command = AsyncCommand::new("yt-dlp");
    
    // Add format selection based on requested format and quality
    if format == "mp3" {
        command.arg("-f")
               .arg("bestaudio[ext=m4a]")
               .arg("--extract-audio")
               .arg("--audio-format")
               .arg("mp3");
    } else {
        command.arg("-f");
        
        let quality_code = match quality {
            Some("480") => "best[height<=480]",
            Some("720") => "best[height<=720]",
            Some("1080") => "best[height<=1080]",
            _ => "bestvideo+bestaudio[ext=m4a]/best",
        };
        
        command.arg(quality_code);
    }
    
    // Add output path
    command.arg("-o").arg(&output_path);
    
    // Handle playlist options
    if use_playlist {
        command.arg("--yes-playlist");
        println!("{}", "Playlist mode enabled - will download all videos in playlist".yellow());
    } else {
        command.arg("--no-playlist");
    }
    
    // Add subtitles if requested
    if download_subtitles {
        command.arg("--write-subs").arg("--sub-langs").arg("all");
        println!("{}", "Subtitles will be downloaded if available".blue());
    }
    
    // Process start and end times
    if start_time.is_some() || end_time.is_some() {
        let mut time_args = String::new();
        
        if let Some(start) = start_time {
            time_args.push_str(&format!("-ss {} ", start));
        }
        
        if let Some(end) = end_time {
            time_args.push_str(&format!("-to {} ", end));
        }
        
        if !time_args.is_empty() {
            command.arg("--postprocessor-args").arg(format!("ffmpeg:{}", time_args.trim()));
        }
    }
    
    // Add throttling and retry options to avoid detection
    command.arg("--socket-timeout").arg("30");
    command.arg("--retries").arg("10");
    command.arg("--fragment-retries").arg("10");
    command.arg("--throttled-rate").arg("100K");
    
    // Add progress output format for parsing
    command.arg("--newline");
    command.arg("--progress-template").arg("download:%(progress.downloaded_bytes)s/%(progress.total_bytes)s");
    
    // Add user agent to avoid detection
    command.arg("--user-agent")
           .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
    
    // Try to use cookies if available (optional)
    if let Ok(home) = std::env::var("HOME") {
        let cookies_path = format!("{}/.cookies.txt", home);
        if std::path::Path::new(&cookies_path).exists() {
            command.arg("--cookies").arg(cookies_path);
        }
    }
    
    // Add the URL last
    command.arg(url);
    
    // Execute the command
    println!("{}", "Starting download...".green());
    
    // Set up pipes for stdout and stderr
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    
    // Spawn the command
    let mut child = command.spawn().map_err(|e| {
        eprintln!("{}", "Failed to execute yt-dlp command.".red());
        AppError::IoError(e)
    })?;
    
    // Process stdout to update progress bar
    if let Some(stdout) = child.stdout.take() {
        let stdout_reader = BufReader::new(stdout);
        let mut lines = stdout_reader.lines();
        let pb_clone = Arc::clone(&pb);
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if line.starts_with("download:") {
                    if let Some(progress_str) = line.strip_prefix("download:") {
                        let parts: Vec<&str> = progress_str.split('/').collect();
                        if parts.len() == 2 {
                            // Try to parse downloaded and total bytes
                            if let (Ok(downloaded), Ok(total)) = (
                                parts[0].trim().parse::<u64>(),
                                parts[1].trim().parse::<u64>(),
                            ) {
                                if total > 0 {
                                    let percentage = (downloaded as f64 / total as f64 * 100.0) as u64;
                                    pb_clone.set_position(percentage);
                                }
                            }
                        }
                    }
                } else {
                    // Print other output from yt-dlp
                    println!("{}", line);
                }
            }
        });
    }
    
    // Process stderr to show errors
    if let Some(stderr) = child.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        let mut lines = stderr_reader.lines();
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("{}", line.red());
            }
        });
    }
    
    // Wait for the command to finish
    let status = child.wait().await.map_err(|e| {
        eprintln!("{}", "Failed to wait for yt-dlp to complete.".red());
        AppError::IoError(e)
    })?;
    
    // Finish the progress bar
    pb.finish_with_message("Download completed");
    
    // Check if command succeeded
    if !status.success() {
        return Err(AppError::DownloadError(
            "yt-dlp command failed. Please verify the URL and options provided.".to_string(),
        ));
    }
    
    // Send desktop notification
    Notification::new()
        .summary("Download Complete")
        .body(&format!("{} file downloaded successfully.", format.to_uppercase()))
        .show()
        .map_err(|e| AppError::General(format!("Failed to show notification: {}", e)))?;
    
    println!(
        "{} {} {}",
        "Download completed successfully.".green(),
        format.to_uppercase(),
        "file saved.".green()
    );
    
    Ok(())
}