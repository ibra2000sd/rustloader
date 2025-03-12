#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rustloader::downloader::download_video_free;
use rustloader::license::{activate_license, is_pro_version, LicenseStatus};
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
    // Check if download is already in progress - Fix for Send issue
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
    
    // Create output directory reference
    let output_dir_ref = output_dir.as_ref(); // This gives us an Option<&String>
    
    // Always use force_download = false in GUI
    let force_download = false;
    let bitrate = None;
    
    // Use a custom download function that takes our progress emitter
    // Note: This is a simplified example - in a real implementation, you would
    // need to modify rustloader's download_video_free function to accept a progress callback
    
    // Let's assume we have a modified version of download_video_free that accepts a progress callback
    // In reality, you'll need to adapt your existing download function to accept and use this callback
    let result = download_video_with_progress(
        &url,
        quality_ref,
        &format,
        start_time_ref,
        end_time_ref,
        use_playlist,
        download_subtitles,
        output_dir_ref, // This is an Option<&String> which matches what download_video_free expects
        force_download,
        bitrate,
        progress_emitter,
    ).await;
    
    // Reset download in progress flag
    {
        let mut download_in_progress = state.download_in_progress.lock().unwrap();
        *download_in_progress = false;
    }
    
    // Convert the result to our return type
    match result {
        Ok(_) => Ok("Download completed successfully".to_string()),
        Err(e) => Err(format!("Download failed: {}", e)),
    }
}

// This function wraps the actual download function and handles progress updates
// In a real implementation, you would modify your download_video_free to accept a progress callback
async fn download_video_with_progress(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&String>,
    end_time: Option<&String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<&String>, // Changed to match download_video_free's signature
    force_download: bool,
    bitrate: Option<&String>,
    progress_emitter: Arc<ProgressEmitter>,
) -> Result<(), AppError> {
    // Create a closure that will emit progress events
    let progress_callback = move |downloaded: u64, total: u64| -> bool {
        progress_emitter.emit_progress(downloaded, total)
    };
    // In a real implementation, you would pass the progress_emitter to download_video_free
    // and have it call progress_emitter.emit_progress() with the downloaded and total bytes
    
    // For now, we'll just call download_video_free directly, without progress updates
    // This is a simplification - in reality, you need to modify the download_video_free function
    
    // Example of a progress callback that we would pass to the modified download_video_free
    let _progress_callback = move |downloaded: u64, total: u64| -> bool {
        progress_emitter.emit_progress(downloaded, total)
    };
    
    // Call the actual download function
    // In a real implementation, you would pass the progress callback to download_video_free
    download_video_free(
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
        Some(progress_callback), // Pass the callback
    ).await
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