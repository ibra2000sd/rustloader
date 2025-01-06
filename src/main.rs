use clap::{Arg, Command}; // Command-line argument parsing
use colored::*; // Colored terminal output
use notify_rust::Notification; // Desktop notifications
use std::fs::{self}; // File handling
use std::io; // Input/output operations
use std::path::PathBuf; // Path operations
use std::process::{Command as ShellCommand, Stdio}; // Shell command execution
use home::home_dir; // Get home directory

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program_name = "rustloader"; // Application name

    fn is_ytdlp_installed() -> Result<bool, Box<dyn std::error::Error>> {
        let output = if cfg!(target_os = "windows") {
            ShellCommand::new("where")
                .arg("yt-dlp")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        } else {
            ShellCommand::new("which")
                .arg("yt-dlp")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };
    
        Ok(output.map(|status| status.success()).unwrap_or(false))
    }

    if !is_ytdlp_installed()? {
    eprintln!("{}", "yt-dlp is not installed. Please install it and try again.".red());
    return Err("yt-dlp is required.".into());
}
    

    // Check if ffmpeg is installed
    if !is_ffmpeg_installed()? {
        println!("{}", "ffmpeg is not installed. Would you like to install it now? (y/n):".yellow());
        let mut input = String::new();
        io::stdin().read_line(&mut input)?; // Read user input
        if input.trim().eq_ignore_ascii_case("y") {
            install_ffmpeg()?; // Install ffmpeg if the user agrees
        } else {
            println!("{}", "Please install ffmpeg manually and try again.".red());
            return Ok(());
        }
    }

    // Parse CLI arguments
    let matches = Command::new(program_name)
        .version("1.0")
        .author("Ibrahim Mohamed")
        .about("Video downloader for any type of content")
        .arg(
            Arg::new("url")
                .help("The URL of the video to download")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("quality")
                .long("quality")
                .short('q')
                .help("Specify the desired quality (480, 720, 1080)")
                .value_parser(["480", "720", "1080"]),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .help("Specify the format (mp4 or mp3)")
                .value_parser(["mp4", "mp3"]),
        )
        .arg(
            Arg::new("start-time")
                .long("start-time")
                .short('s')
                .help("Specify the start time of the clip (e.g., 00:01:00)")
                .value_name("START_TIME"),
        )
        .arg(
            Arg::new("end-time")
                .long("end-time")
                .short('e')
                .help("Specify the end time of the clip (e.g., 00:02:00)")
                .value_name("END_TIME"),
        )
        .get_matches();

    let url = matches.get_one::<String>("url").unwrap();
    let quality = matches.get_one::<String>("quality").map(|q| q.as_str());
    let format = matches.get_one::<String>("format").map(|f| f.as_str()).unwrap_or("mp4");
    let start_time = matches.get_one::<String>("start-time");
    let end_time = matches.get_one::<String>("end-time");

    if let Err(e) = download_video(url, quality, format, start_time, end_time) {
        eprintln!("Error downloading video: {}", e);
    }

    Ok(())
}

// Check if ffmpeg is installed
fn is_ffmpeg_installed() -> Result<bool, Box<dyn std::error::Error>> {
    let output = if cfg!(target_os = "windows") {
        ShellCommand::new("where")
            .arg("ffmpeg")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        ShellCommand::new("which")
            .arg("ffmpeg")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    };

    Ok(output.map(|status| status.success()).unwrap_or(false))
}

// Install ffmpeg
fn install_ffmpeg() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Installing ffmpeg...".blue());

    #[cfg(target_os = "macos")]
    {
        let status = ShellCommand::new("brew")
            .arg("install")
            .arg("ffmpeg")
            .status()?;

        if status.success() {
            println!("{}", "ffmpeg installed successfully.".green());
        } else {
            eprintln!("{}", "Failed to install ffmpeg. Please install it manually.".red());
            return Err("ffmpeg installation failed.".into());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = ShellCommand::new("sudo")
            .arg("apt")
            .arg("install")
            .arg("-y")
            .arg("ffmpeg")
            .status()?;

        if status.success() {
            println!("{}", "ffmpeg installed successfully.".green());
        } else {
            eprintln!("{}", "Failed to install ffmpeg. Please install it manually.".red());
        }
    }

    #[cfg(target_os = "windows")]
    {
        println!("{}", "Automatic installation of ffmpeg is not supported on Windows.".yellow());
        println!("{}", "Please download and install ffmpeg manually from: https://ffmpeg.org/download.html".yellow());
    }

    Ok(())
}

// Download a video or audio clip
fn download_video(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&String>,
    end_time: Option<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}: {}", "Downloading".blue(), url);
    println!("Download started. You can continue your work.");

    // Initialize download directory
    let folder_type = if format == "mp3" { "audio" } else { "videos" };
    let download_dir = initialize_download_dir("rustloader", folder_type)?;

    // Determine output filename format
    let output = download_dir.join(format!("%(title)s.{}", format)).to_str().unwrap().to_string();

    // Build yt-dlp command
    let mut command = ShellCommand::new("yt-dlp");
    command.arg("-f");

    // Handle format-specific settings
    if format == "mp3" {
        command.arg("bestaudio[ext=m4a]").arg("--extract-audio").arg("--audio-format").arg("mp3");
    } else {
        let quality_code = match quality {
            Some("480") => "best[height<=480]",
            Some("720") => "best[height<=720]",
            Some("1080") => "best[height<=1080]",
            _ => "bestvideo+bestaudio[ext=m4a]/best",
        };
        command.arg(quality_code);
    }

    // Set output directory and URL
    command.arg("-o").arg(&output).arg(url);

    // Validate and set start and end times
    let (start, end) = match (start_time, end_time) {
        (Some(start), None) => (Some(start.clone()), Some("00:00:00".to_string())), // Start time only
        (None, Some(end)) => (Some("00:00:00".to_string()), Some(end.clone())),     // End time only
        (Some(start), Some(end)) => (Some(start.clone()), Some(end.clone())),       // Both start and end times
        (None, None) => (None, None),                                               // Neither specified
    };

    // Validate time formats
    if let Some(start) = &start {
        if !is_valid_time_format(start) {
            eprintln!("{}", "Invalid start time format. Please use HH:MM:SS.".red());
            return Err("Invalid start time format.".into());
        }
    }
    if let Some(end) = &end {
        if !is_valid_time_format(end) {
            eprintln!("{}", "Invalid end time format. Please use HH:MM:SS.".red());
            return Err("Invalid end time format.".into());
        }
    }

    // Display warning messages if only one of the times is provided
    if start_time.is_some() && end_time.is_none() {
        println!("{}", "Start time specified without end time. Assuming end of video.".yellow());
    } else if end_time.is_some() && start_time.is_none() {
        println!("{}", "End time specified without start time. Assuming start of video.".yellow());
    }

    // Add start and end times to yt-dlp command if specified
    if let (Some(start), Some(end)) = (start, end) {
        command.arg("--postprocessor-args").arg(format!("-ss {} -to {}", start, end));
    }

    // Execute the command
    let status = command.status().map_err(|e| {
        eprintln!("{}", "Failed to execute yt-dlp command. Please check if yt-dlp is installed and accessible.".red());
        eprintln!("Error details: {:?}", e);
        e
    })?;

    // Check the command execution result
    if !status.success() {
        eprintln!("{}", "yt-dlp command failed. Please verify the URL and options provided.".red());
        return Err("yt-dlp execution failed.".into());
    }

    // Notify user of successful download
    Notification::new()
        .summary("Download Complete")
        .body(&format!("{} file downloaded successfully.", format.to_uppercase()))
        .show()?;
    println!("{} {} {}", "Download completed successfully.".green(), format, "file.".green());

    Ok(())
}

// Helper function to validate time format
fn is_valid_time_format(time: &str) -> bool {
    let re = regex::Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap();
    re.is_match(time)
}

// Initialize the download directory
fn initialize_download_dir(program_name: &str, file_type: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let download_dir = match home_dir() {
        Some(mut path) => {
            path.push("Downloads");
            path.push(program_name);
            path.push(file_type);
            path
        }
        None => {
            eprintln!("{}", "Failed to find the home directory.".red());
            return Err("Unable to locate home directory.".into());
        }
    };

    if !download_dir.exists() {
        fs::create_dir_all(&download_dir).map_err(|e| {
            eprintln!("{}: {:?}", "Failed to create download directory".red(), e);
            e
        })?;
        println!("{} {:?}", "Created directory:".green(), download_dir);
    }

    Ok(download_dir)
}