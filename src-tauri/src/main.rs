#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rustloader::downloader::{download_video_free, download_video_pro};
use rustloader::license::{activate_license, is_pro_version, LicenseStatus};
use rustloader::ffmpeg_wrapper;
use rustloader::error::AppError;
use std::sync::{Arc, Mutex};
use tauri::{command, Window, State};

// Define app state to share between commands
struct AppState {
    download_in_progress: Mutex<bool>,
}

// Progress callback implementation
struct ProgressEmitter {
    window: Window,
}

impl ProgressEmitter {
    fn new(window: Window) -> Self {
        Self { window }
    }

    fn emit_progress(&self, downloaded: u64, total: u64) -> bool {
        let percentage = if total > 0 { 
            (downloaded * 100) / total 
        } else { 
            0 
        };
        
        // Emit the progress event to the frontend
        match self.window.emit("download-progress", percentage) {
            Ok(_) => true, // Continue download
            Err(e) => {
                eprintln!("Failed to emit progress: {}", e);
                true // Continue download even if event emission fails
            }
        }
    }
}

// Command to check license status
#[command]
fn check_license() -> String {
    if is_pro_version() {
        "pro".to_string()
    } else {
        "free".to_string()
    }
}

// Command to activate a license key
#[command]
fn activate_license_key(license_key: String, email: String) -> Result<String, String> {
    match activate_license(&license_key, &email) {
        Ok(LicenseStatus::Pro(_)) => Ok("License activated successfully".to_string()),
        Ok(LicenseStatus::Invalid(reason)) => Err(format!("Invalid license: {}", reason)),
        Ok(LicenseStatus::Free) => Err("Activation failed".to_string()),
        Err(e) => Err(format!("Activation error: {}", e)),
    }
}

// Command to download a video
#[command]
async fn download_video(
    window: Window,
    url: String,
    quality: Option<String>,
    format: String,
    start_time: Option<String>,
    end_time: Option<String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Initialize FFmpeg
    if let Err(e) = ffmpeg_wrapper::init() {
        return Err(format!("Error initializing FFmpeg: {}", e));
    }
    
    // Check if download is already in progress
    {
        let mut download_in_progress = state.download_in_progress.lock().unwrap();
        
        if *download_in_progress {
            return Err("A download is already in progress".to_string());
        }
        
        // Mark download as in progress
        *download_in_progress = true;
    } // Lock is dropped here before the await
    
    // Create progress emitter
    let progress_emitter = Arc::new(ProgressEmitter::new(window.clone()));
    
    // Convert option strings to option refs for the download function
    let quality_ref = quality.as_deref();
    let start_time_ref = start_time.as_ref();
    let end_time_ref = end_time.as_ref();
    let output_dir_ref = output_dir.as_ref();
    
    // Create a progress callback
    let progress_callback = move |downloaded: u64, total: u64| -> bool {
        progress_emitter.emit_progress(downloaded, total)
    };
    
    // Check if Pro version
    let is_pro = is_pro_version();
    
    // Always use force_download = false in GUI
    let force_download = false;
    let bitrate = None;
    
    // Use appropriate download function based on license
    let result = if is_pro {
        // Pro version
        download_video_pro(
            &url,
            quality_ref,
            &format,
            start_time_ref,
            end_time_ref,
            use_playlist,
            download_subtitles,
            output_dir_ref,
            force_download,
            bitrate,
            Some(progress_callback),
        ).await
    } else {
        // Free version
        download_video_free(
            &url,
            quality_ref,
            &format,
            start_time_ref,
            end_time_ref,
            use_playlist,
            download_subtitles,
            output_dir_ref,
            force_download,
            bitrate,
            Some(progress_callback),
        ).await
    };
    
    // Reset download in progress flag
    {
        let mut download_in_progress = state.download_in_progress.lock().unwrap();
        *download_in_progress = false;
    }
    
    // Convert the result to our return type
    match result {
        Ok(_) => Ok("Download completed successfully".to_string()),
        Err(e) => {
            match e {
                AppError::DailyLimitExceeded => {
                    Err("Daily download limit exceeded for free version. Upgrade to Pro for unlimited downloads.".to_string())
                },
                AppError::PremiumFeature(feature) => {
                    Err(format!("Premium feature required: {}. Upgrade to Pro to access this feature.", feature))
                },
                AppError::DownloadError(msg) if msg.contains("HTTP 416") => {
                    Err("File already exists. Please try again with a different filename.".to_string())
                },
                _ => Err(format!("Download failed: {}", e)),
            }
        }
    }
}

// Main function to run the Tauri application
fn main() {
    tauri::Builder::default()
        .manage(AppState {
            download_in_progress: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            check_license,
            activate_license_key,
            download_video,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}