// src/ffmpeg_wrapper.rs

use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::io::{BufRead, BufReader};
use std::thread;
use std::sync::mpsc;
use std::fs;

pub type ProgressCallback = Arc<dyn Fn(u64, u64) -> bool + Send + Sync>;

/// Configuration for FFmpeg operations
pub struct FFmpegConfig {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub format: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub bitrate: Option<String>,
}

/// Initialize FFmpeg by checking its availability
pub fn init() -> Result<(), AppError> {
    // Check if ffmpeg is available
    let output = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| AppError::IoError(e))?;
    
    if !output.success() {
        return Err(AppError::MissingDependency("FFmpeg is not installed or not in PATH".to_string()));
    }
    
    Ok(())
}

/// Extract audio from video file using ffmpeg command-line
pub fn extract_audio(
    config: &FFmpegConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<(), AppError> {
    // Make sure the output directory exists
    if let Some(parent) = config.output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::IoError(e))?;
    }
    
    // Get input file duration for progress tracking
    let duration_secs = get_video_duration(&config.input_path)?;
    let total_frames = duration_secs * 30; // Approximate frames at 30fps
    
    // Build FFmpeg command for audio extraction
    let mut command = Command::new("ffmpeg");
    
    // Add input file
    command.arg("-i").arg(&config.input_path);
    
    // Add start time if specified
    if let Some(start) = &config.start_time {
        command.arg("-ss").arg(start);
    }
    
    // Add end time if specified
    if let Some(end) = &config.end_time {
        command.arg("-to").arg(end);
    }
    
    // Add audio format and bitrate
    command.arg("-vn"); // No video
    
    // Set audio format
    command.arg("-f").arg("mp3");
    
    // Set audio bitrate
    if let Some(bitrate) = &config.bitrate {
        command.arg("-b:a").arg(bitrate);
    } else {
        command.arg("-b:a").arg("128k"); // Default bitrate
    }
    
    // Add output file
    command.arg("-y") // Overwrite output file if it exists
           .arg(&config.output_path);
    
    // Add progress reporting
    command.arg("-progress").arg("pipe:1");
    
    // Set up standard streams
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    
    // Create a channel for progress updates
    let (tx, rx) = mpsc::channel();
    
    // Run FFmpeg command
    let mut child = command.spawn()
        .map_err(|e| AppError::IoError(e))?;
    
    // Set up progress monitoring thread
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let sender = tx.clone();
        
        thread::spawn(move || {
            let mut current_frame = 0;
            
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.starts_with("frame=") {
                        if let Some(frame_str) = line.strip_prefix("frame=") {
                            if let Ok(frame_num) = frame_str.trim().parse::<u64>() {
                                current_frame = frame_num;
                                
                                // Send progress update
                                let _ = sender.send((current_frame, total_frames));
                            }
                        }
                    }
                }
            }
        });
    }
    
    // Set up stderr monitoring thread
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        
        thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    // Just log FFmpeg output for now
                    eprintln!("FFmpeg: {}", line);
                }
            }
        });
    }
    
    // Create a thread to process progress updates
    if let Some(callback) = progress_callback {
        thread::spawn(move || {
            while let Ok((frame, total)) = rx.recv() {
                if !callback(frame, total) {
                    // Callback returned false - cancel operation
                    // Unfortunately we can't easily cancel the FFmpeg process mid-operation
                    // But we can stop reporting progress
                    break;
                }
            }
        });
    }
    
    // Wait for FFmpeg to finish
    let status = child.wait()
        .map_err(|e| AppError::IoError(e))?;
    
    if status.success() {
        Ok(())
    } else {
        Err(AppError::General(format!("FFmpeg exited with status: {}", status)))
    }
}

/// Convert video to different quality using ffmpeg command-line
pub fn convert_video(
    config: &FFmpegConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<(), AppError> {
    // Make sure the output directory exists
    if let Some(parent) = config.output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::IoError(e))?;
    }
    
    // Get input file duration for progress tracking
    let duration_secs = get_video_duration(&config.input_path)?;
    let total_frames = duration_secs * 30; // Approximate frames at 30fps
    
    // Build FFmpeg command for video conversion
    let mut command = Command::new("ffmpeg");
    
    // Add input file
    command.arg("-i").arg(&config.input_path);
    
    // Add start time if specified
    if let Some(start) = &config.start_time {
        command.arg("-ss").arg(start);
    }
    
    // Add end time if specified
    if let Some(end) = &config.end_time {
        command.arg("-to").arg(end);
    }
    
    // Set video resolution based on quality
    match config.format.as_str() {
        "480" => { command.arg("-vf").arg("scale=-1:480"); }
        "720" => { command.arg("-vf").arg("scale=-1:720"); }
        "1080" => { command.arg("-vf").arg("scale=-1:1080"); }
        "2160" => { command.arg("-vf").arg("scale=-1:2160"); }
        _ => { /* Use input resolution */ }
    };
    
    // Set video codec and quality
    command.arg("-c:v").arg("libx264");
    
    // Set CRF (quality) - lower is better, 23 is default
    command.arg("-crf").arg("23");
    
    // Set preset - slower = better compression
    command.arg("-preset").arg("medium");
    
    // Set audio codec
    command.arg("-c:a").arg("aac");
    
    // Set audio bitrate
    if let Some(bitrate) = &config.bitrate {
        command.arg("-b:a").arg(bitrate);
    } else {
        command.arg("-b:a").arg("128k"); // Default bitrate
    }
    
    // Add output format
    command.arg("-f").arg("mp4");
    
    // Add output file
    command.arg("-y") // Overwrite output file if it exists
           .arg(&config.output_path);
    
    // Add progress reporting
    command.arg("-progress").arg("pipe:1");
    
    // Set up standard streams
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    
    // Create a channel for progress updates
    let (tx, rx) = mpsc::channel();
    
    // Run FFmpeg command
    let mut child = command.spawn()
        .map_err(|e| AppError::IoError(e))?;
    
    // Set up progress monitoring thread
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let sender = tx.clone();
        
        thread::spawn(move || {
            let mut current_frame = 0;
            
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.starts_with("frame=") {
                        if let Some(frame_str) = line.strip_prefix("frame=") {
                            if let Ok(frame_num) = frame_str.trim().parse::<u64>() {
                                current_frame = frame_num;
                                
                                // Send progress update
                                let _ = sender.send((current_frame, total_frames));
                            }
                        }
                    }
                }
            }
        });
    }
    
    // Set up stderr monitoring thread
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        
        thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    // Just log FFmpeg output for now
                    eprintln!("FFmpeg: {}", line);
                }
            }
        });
    }
    
    // Create a thread to process progress updates
    if let Some(callback) = progress_callback {
        thread::spawn(move || {
            while let Ok((frame, total)) = rx.recv() {
                if !callback(frame, total) {
                    // Callback returned false - cancel operation
                    // Unfortunately we can't easily cancel the FFmpeg process mid-operation
                    // But we can stop reporting progress
                    break;
                }
            }
        });
    }
    
    // Wait for FFmpeg to finish
    let status = child.wait()
        .map_err(|e| AppError::IoError(e))?;
    
    if status.success() {
        Ok(())
    } else {
        Err(AppError::General(format!("FFmpeg exited with status: {}", status)))
    }
}

/// Check if a file already exists and handle duplicates
pub fn handle_duplicate_file(path: &Path) -> Result<PathBuf, AppError> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    
    // Get filename and extension
    let file_stem = path.file_stem()
        .ok_or_else(|| AppError::PathError("Invalid filename".to_string()))?
        .to_string_lossy();
    
    let extension = path.extension()
        .map(|ext| ext.to_string_lossy().to_string())
        .unwrap_or_default();
    
    // Create a new filename with timestamp
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S");
    
    let new_filename = if extension.is_empty() {
        format!("{}_{}", file_stem, timestamp)
    } else {
        format!("{}_{}.{}", file_stem, timestamp, extension)
    };
    
    // Create new path
    let new_path = path.with_file_name(new_filename);
    
    Ok(new_path)
}

/// Get the duration of a video file in seconds
fn get_video_duration(path: &Path) -> Result<u64, AppError> {
    // Use ffprobe to get the duration
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .map_err(|e| AppError::IoError(e))?;
    
    if !output.status.success() {
        return Err(AppError::General("Failed to get video duration".to_string()));
    }
    
    let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let duration_secs = duration_str.parse::<f64>()
        .map_err(|_| AppError::General("Failed to parse video duration".to_string()))?;
    
    Ok(duration_secs.round() as u64)
}