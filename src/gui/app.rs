//! Main GUI application
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::database::{initialize_database, DatabaseManager};
use crate::extractor::VideoInfo;
use crate::gui::clipboard;
use crate::gui::integration::{BackendBridge, ProgressUpdate};
use crate::queue::TaskStatus;
use crate::utils::config::{AppSettings, VideoQuality};
use anyhow::Result;
use iced::{Application, Command, Element, Subscription, Theme};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Main application state
pub struct RustloaderApp {
    // Core components
    backend: std::sync::Arc<std::sync::Mutex<BackendBridge>>,
    db_manager: Arc<DatabaseManager>,
    // Keep a long-lived runtime so backend tasks stay alive
    runtime: Arc<Runtime>,

    // UI State
    current_view: View,
    url_input: String,
    status_message: String,

    // Download tasks
    active_downloads: Vec<DownloadTaskUI>,

    // Settings
    download_location: String,
    max_concurrent: usize,
    segments_per_download: usize,
    quality: VideoQuality,

    // Flags
    is_extracting: bool,
    url_error: Option<String>,
}

/// Application view
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Main,
    Settings,
}

/// Download task UI representation
#[derive(Debug, Clone)]
pub struct DownloadTaskUI {
    pub id: String,
    pub title: String,
    pub url: String,
    pub progress: f32,  // 0.0 to 1.0
    pub speed: f64,     // bytes per second
    pub status: String, // "Downloading", "Paused", "Completed", etc.
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub eta_seconds: Option<u64>,
    pub file_path: Option<String>, // Path to the downloaded file
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    // Input events
    UrlInputChanged(String),
    DownloadButtonPressed,
    PasteFromClipboard,
    ClearUrlInput,

    // Extract events
    ExtractionStarted,
    ExtractionCompleted(Result<VideoInfo, String>),

    // Download events
    DownloadStarted(String), // task_id
    DownloadStartedWithInfo(String, VideoInfo),
    DownloadProgress(String, DownloadProgressData),
    DownloadCompleted(String),
    DownloadFailed(String, String),

    // Queue control
    PauseDownload(String),
    ResumeDownload(String),
    CancelDownload(String),
    RemoveCompleted(String),
    ClearAllCompleted,
    RetryDownload(String),
    OpenFile(String),
    OpenDownloadFolder(String),

    // View navigation
    SwitchToMain,
    SwitchToSettings,

    // Settings
    DownloadLocationChanged(String),
    BrowseDownloadLocation,
    MaxConcurrentChanged(usize),
    SegmentsChanged(usize),
    QualityChanged(String),
    SaveSettings,

    // System
    Tick, // For periodic UI updates
}

/// Download progress data
#[derive(Debug, Clone)]
pub struct DownloadProgressData {
    pub progress: f32,
    pub speed: f64,
    pub downloaded: u64,
    pub total: u64,
    pub eta: Option<u64>,
}

impl Application for RustloaderApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        // Initialize settings
        let mut settings = AppSettings::default();

        // Initialize database
        let db_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rustloader")
            .join("rustloader.db");

        // Create directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let db_path_str = db_path.to_string_lossy().to_string();

        // Create a single runtime and keep it alive for the app lifetime
        let rt = Runtime::new().expect("Failed to create tokio runtime");

        let db_pool = rt
            .block_on(initialize_database(&db_path_str))
            .expect("Failed to initialize database");

        let db_manager = Arc::new(DatabaseManager::new(db_pool));

        // Load settings from database
        if let Ok(loaded_settings) = rt.block_on(load_settings_from_db(&db_manager)) {
            settings = loaded_settings;
        }

        // Initialize backend on the same runtime and wrap it in a sync Arc<Mutex> for GUI access
        let backend_bridge = rt
            .block_on(BackendBridge::new(settings.clone()))
            .expect("Failed to initialize backend");
        let backend = std::sync::Arc::new(std::sync::Mutex::new(backend_bridge));
        let runtime = Arc::new(rt);

        let app = Self {
            backend,
            db_manager,
            runtime,
            current_view: View::Main,
            url_input: String::new(),
            status_message: "Ready".to_string(),
            active_downloads: Vec::new(),
            download_location: settings.download_location.to_string_lossy().to_string(),
            max_concurrent: settings.max_concurrent,
            segments_per_download: settings.segments,
            quality: settings.quality,
            is_extracting: false,
            url_error: None,
        };

        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("Rustloader - High-Performance Video Downloader")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Input events
            Message::UrlInputChanged(url) => {
                self.url_input = url;
                self.url_error = None; // Clear error when user types
                Command::none()
            }

            Message::DownloadButtonPressed => {
                if !self.url_input.is_empty() && !self.is_extracting {
                    self.is_extracting = true;
                    self.status_message = "Extracting video information...".to_string();

                    let url = self.url_input.clone();
                    // Call backend extractor via the stored runtime and backend bridge
                    let backend = std::sync::Arc::clone(&self.backend);
                    let runtime = std::sync::Arc::clone(&self.runtime);

                    Command::perform(
                        async move {
                            // Use the stored runtime to spawn an async task that calls the backend.
                            // Communicate result back via a oneshot channel to avoid blocking runtime threads.
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            let backend_clone = backend.clone();
                            let url_clone = url.clone();

                            // Run a dedicated thread that creates its own runtime to call the backend.
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new()
                                    .expect("Failed to create runtime for extractor thread");
                                let res = rt.block_on(async move {
                                    // FIX BUG-001: Clone bridge while holding lock, then release before await
                                    let bridge_result = {
                                        match backend_clone.lock() {
                                            Ok(bridge) => {
                                                // Clone the BackendBridge (cheap Arc clones)
                                                Ok(bridge.clone())
                                            }
                                            Err(e) => Err(format!("Backend mutex poisoned: {}", e)),
                                        }
                                    }; // Lock is dropped here!

                                    // Now use cloned bridge with await (no lock held)
                                    match bridge_result {
                                        Ok(bridge) => bridge.extract_video_info(&url_clone).await,
                                        Err(e) => Err(e),
                                    }
                                });
                                let _ = tx.send(res);
                            });

                            match rx.await {
                                Ok(res) => res,
                                Err(_) => Err("Extractor task canceled".to_string()),
                            }
                        },
                        |result: Result<VideoInfo, String>| {
                            Message::ExtractionCompleted(result.map_err(|e| e.to_string()))
                        },
                    )
                } else {
                    Command::none()
                }
            }

            Message::PasteFromClipboard => {
                match clipboard::get_clipboard_content() {
                    Ok(content) => {
                        self.url_input = content;
                        self.status_message = "URL pasted from clipboard".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to paste from clipboard: {}", e);
                    }
                }
                Command::none()
            }

            Message::ClearUrlInput => {
                self.url_input.clear();
                Command::none()
            }

            // Extract events
            Message::ExtractionStarted => {
                self.is_extracting = true;
                self.status_message = "Extracting video information...".to_string();
                Command::none()
            }

            Message::ExtractionCompleted(result) => {
                self.is_extracting = false;

                match result {
                    Ok(video_info) => {
                        // Prepare output path
                        let output_path = PathBuf::from(&self.download_location)
                            .join(format!("{}.mp4", sanitize_filename(&video_info.title)));

                        // Clear URL input, error, and update status while we start the download
                        self.url_input.clear();
                        self.url_error = None;
                        self.status_message =
                            format!("Starting download: {}", video_info.title.clone());

                        // Kick off start_download on backend and include video_info so UI can create an entry
                        let backend = std::sync::Arc::clone(&self.backend);
                        let runtime = std::sync::Arc::clone(&self.runtime);
                        let vi_clone = video_info.clone();
                        let out_clone = output_path.clone();

                        eprintln!(
                            "⏳ GUI: preparing to call backend.start_download for '{}'",
                            vi_clone.title
                        );
                        return Command::perform(
                            async move {
                                // Spawn the backend start_download on the app runtime and use oneshot to receive the task id.
                                let (tx, rx) = tokio::sync::oneshot::channel();
                                let backend_clone = backend.clone();
                                let vi = vi_clone.clone();
                                let out = out_clone.clone();

                                // Spawn a blocking thread with its own runtime to start the download.
                                let vi_for_call = vi.clone();
                                let vi_title = vi.title.clone();
                                std::thread::spawn(move || {
                                    eprintln!(
                                        "⏳ [GUI THREAD] calling backend.start_download for '{}'",
                                        vi_title
                                    );
                                    let rt = tokio::runtime::Runtime::new().expect(
                                        "Failed to create runtime for start_download thread",
                                    );
                                    let res = rt.block_on(async move {
                                        // FIX BUG-001: Clone bridge while holding lock, then release before await
                                        let bridge_result = {
                                            match backend_clone.lock() {
                                                Ok(bridge) => {
                                                    // Clone the BackendBridge (cheap Arc clones)
                                                    Ok(bridge.clone())
                                                }
                                                Err(e) => {
                                                    Err(format!("Backend mutex poisoned: {}", e))
                                                }
                                            }
                                        }; // Lock is dropped here!

                                        // Now use cloned bridge with await (no lock held)
                                        match bridge_result {
                                            Ok(bridge) => {
                                                bridge
                                                    .start_download(
                                                        vi_for_call.clone(),
                                                        out.clone(),
                                                        None,
                                                    )
                                                    .await
                                            }
                                            Err(e) => Err(e),
                                        }
                                    });
                                    eprintln!("⏳ [GUI THREAD] backend.start_download returned for '{}': {:?}", vi_title, res.as_ref().map(|s| s.as_str()).unwrap_or("Err"));
                                    let _ = tx.send(res);
                                });

                                let res = match rx.await {
                                    Ok(r) => r,
                                    Err(_) => Err("Start download task canceled".to_string()),
                                };

                                eprintln!(
                                    "⏳ GUI: received start_download result for '{}': {:?}",
                                    vi_clone.title,
                                    res.as_ref().map(|s| s.as_str()).unwrap_or("Err")
                                );

                                (res, vi_clone)
                            },
                            |(res, vi): (Result<String, String>, VideoInfo)| match res {
                                Ok(task_id) => Message::DownloadStartedWithInfo(task_id, vi),
                                Err(e) => Message::DownloadFailed("".to_string(), e),
                            },
                        );
                    }
                    Err(e) => {
                        let friendly_error = make_error_user_friendly(&e);
                        self.url_error = Some(friendly_error.clone());
                        self.status_message = "Ready".to_string();
                    }
                }

                Command::none()
            }

            // Download events
            Message::DownloadStarted(task_id) => {
                // If UI already has an entry with this id, mark it as downloading
                if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                    task.status = "Downloading".to_string();
                }
                Command::none()
            }

            Message::DownloadStartedWithInfo(task_id, video_info) => {
                // Create UI entry now that backend provided the real task id
                let title = video_info.title.clone();
                let url = video_info.url.clone();

                let task_ui = DownloadTaskUI {
                    id: task_id.clone(),
                    title: title.clone(),
                    url,
                    progress: 0.0,
                    speed: 0.0,
                    status: "Queued".to_string(),
                    downloaded_mb: 0.0,
                    total_mb: video_info.filesize.unwrap_or(0) as f64 / (1024.0 * 1024.0),
                    eta_seconds: None,
                    file_path: None,
                };

                self.active_downloads.push(task_ui);
                self.status_message = format!("Added to download queue: {}", title);
                Command::none()
            }

            Message::DownloadProgress(task_id, progress_data) => {
                // ✅ DEBUG BUG-006: Log progress updates
                eprintln!(
                    "📊 [GUI PROGRESS] Task: {} | Progress: {:.1}% | Speed: {:.1} MB/s",
                    task_id,
                    progress_data.progress * 100.0,
                    progress_data.speed / 1024.0 / 1024.0
                );

                if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                    eprintln!("   ✅ Task found, updating UI");
                    task.progress = progress_data.progress;
                    task.speed = progress_data.speed;
                    task.downloaded_mb = progress_data.downloaded as f64 / (1024.0 * 1024.0);
                    task.eta_seconds = progress_data.eta;
                } else {
                    eprintln!("   ❌ Task NOT found in active_downloads!");
                    eprintln!(
                        "   ❌ Current task IDs: {:?}",
                        self.active_downloads
                            .iter()
                            .map(|t| &t.id)
                            .collect::<Vec<_>>()
                    );
                }
                Command::none()
            }

            Message::DownloadCompleted(task_id) => {
                if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                    task.status = "Completed".to_string();
                    task.progress = 1.0;

                    // ✅ FIX: Calculate actual file size for display
                    if task.downloaded_mb > 0.0 {
                        // Size already set by progress updates
                        task.total_mb = task.downloaded_mb;
                    } else if let Some(ref file_path) = task.file_path {
                        // Fallback: try to read actual file size
                        let output_path = std::path::PathBuf::from(file_path);
                        if output_path.exists() {
                            if let Ok(metadata) = std::fs::metadata(&output_path) {
                                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                                task.downloaded_mb = size_mb;
                                task.total_mb = size_mb;
                                eprintln!(
                                    "✅ [COMPLETED] Set file size: {:.1} MB for {}",
                                    size_mb, task_id
                                );
                            } else {
                                eprintln!(
                                    "⚠️  [COMPLETED] Could not read file metadata for {}",
                                    task_id
                                );
                            }
                        } else {
                            eprintln!("⚠️  [COMPLETED] File not found: {:?}", output_path);
                        }
                    } else {
                        eprintln!("⚠️  [COMPLETED] No file_path set for task {}", task_id);
                    }
                }
                self.status_message = "Download completed".to_string();
                Command::none()
            }

            Message::DownloadFailed(task_id, error) => {
                if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                    task.status = "Failed".to_string();
                }
                self.status_message = format!("Download failed: {}", error);
                Command::none()
            }

            // Queue control
            Message::PauseDownload(task_id) => {
                let backend = self.backend.clone();
                let id = task_id.clone();

                // Update local UI state immediately
                if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                    task.status = "Pausing...".to_string();
                }

                Command::perform(
                    async move {
                        // Clone bridge before await to avoid deadlock
                        let bridge_result = {
                            match backend.lock() {
                                Ok(bridge) => Ok(bridge.clone()),
                                Err(e) => Err(format!("Backend mutex poisoned: {}", e)),
                            }
                        }; // Lock dropped here

                        match bridge_result {
                            Ok(mut bridge) => match bridge.pause_download(&id).await {
                                Ok(_) => {
                                    eprintln!("✅ Paused download: {}", id);
                                    Some(())
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to pause task {}: {}", id, e);
                                    None
                                }
                            },
                            Err(e) => {
                                eprintln!("❌ Backend error: {}", e);
                                None
                            }
                        }
                    },
                    |_| Message::Tick, // Trigger UI refresh
                )
            }

            Message::ResumeDownload(task_id) => {
                let backend = self.backend.clone();
                let id = task_id.clone();

                // Update local UI state immediately
                if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                    task.status = "Resuming...".to_string();
                }

                Command::perform(
                    async move {
                        // Clone bridge before await to avoid deadlock
                        let bridge_result = {
                            match backend.lock() {
                                Ok(bridge) => Ok(bridge.clone()),
                                Err(e) => Err(format!("Backend mutex poisoned: {}", e)),
                            }
                        }; // Lock dropped here

                        match bridge_result {
                            Ok(bridge) => match bridge.resume_download(&id).await {
                                Ok(_) => {
                                    eprintln!("✅ Resumed download: {}", id);
                                    Some(())
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to resume task {}: {}", id, e);
                                    None
                                }
                            },
                            Err(e) => {
                                eprintln!("❌ Backend error: {}", e);
                                None
                            }
                        }
                    },
                    |_| Message::Tick, // Trigger UI refresh
                )
            }

            Message::CancelDownload(task_id) => {
                let backend = self.backend.clone();
                let id = task_id.clone();

                // ✅ FIX: Remove from UI immediately instead of showing "Cancelling..."
                self.active_downloads.retain(|t| t.id != task_id);

                Command::perform(
                    async move {
                        // Clone bridge before await to avoid deadlock
                        let bridge_result = {
                            match backend.lock() {
                                Ok(bridge) => Ok(bridge.clone()),
                                Err(e) => Err(format!("Backend mutex poisoned: {}", e)),
                            }
                        }; // Lock dropped here

                        match bridge_result {
                            Ok(bridge) => match bridge.cancel_download(&id).await {
                                Ok(_) => {
                                    eprintln!("✅ Cancelled download: {}", id);
                                    true
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to cancel task {}: {}", id, e);
                                    false
                                }
                            },
                            Err(e) => {
                                eprintln!("❌ Backend error: {}", e);
                                false
                            }
                        }
                    },
                    |_success| Message::Tick, // Just refresh UI
                )
            }

            Message::RemoveCompleted(task_id) => {
                let backend = self.backend.clone();
                let id = task_id.clone();

                // Remove from UI immediately
                if let Some(index) = self.active_downloads.iter().position(|t| t.id == task_id) {
                    self.active_downloads.remove(index);
                }

                // Also remove from backend queue
                Command::perform(
                    async move {
                        let bridge_result = {
                            match backend.lock() {
                                Ok(bridge) => Ok(bridge.clone()),
                                Err(e) => Err(format!("Backend mutex poisoned: {}", e)),
                            }
                        };

                        match bridge_result {
                            Ok(bridge) => {
                                bridge.remove_task(&id).await.ok();
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to remove task from backend: {}", e);
                            }
                        }
                    },
                    |_| Message::Tick,
                )
            }

            Message::ClearAllCompleted => {
                let backend = self.backend.clone();

                // Remove from UI immediately
                self.active_downloads.retain(|t| t.status != "Completed");

                // Also clear from backend queue
                Command::perform(
                    async move {
                        let bridge_result = {
                            match backend.lock() {
                                Ok(bridge) => Ok(bridge.clone()),
                                Err(e) => Err(format!("Backend mutex poisoned: {}", e)),
                            }
                        };

                        match bridge_result {
                            Ok(bridge) => match bridge.clear_completed().await {
                                Ok(_) => eprintln!("✅ Cleared completed tasks from backend"),
                                Err(e) => eprintln!("❌ Failed to clear completed: {}", e),
                            },
                            Err(e) => {
                                eprintln!("❌ Backend error: {}", e);
                            }
                        }
                    },
                    |_| Message::Tick,
                )
            }

            Message::RetryDownload(task_id) => {
                // This would call the backend
                if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                    task.status = "Queued".to_string();
                    task.progress = 0.0;
                }
                Command::none()
            }

            Message::OpenFile(task_id) => {
                // Open the downloaded file with the default application
                if let Some(task) = self.active_downloads.iter().find(|t| t.id == task_id) {
                    if let Some(file_path) = &task.file_path {
                        let path = std::path::PathBuf::from(file_path);
                        if path.exists() {
                            if let Err(e) = open::that(&path) {
                                eprintln!("Failed to open file: {}", e);
                            }
                        } else {
                            eprintln!("File not found: {:?}", path);
                        }
                    } else {
                        eprintln!("File path not available for task: {}", task_id);
                    }
                }
                Command::none()
            }

            Message::OpenDownloadFolder(task_id) => {
                // Open the folder containing the downloaded file
                if let Some(task) = self.active_downloads.iter().find(|t| t.id == task_id) {
                    if let Some(file_path) = &task.file_path {
                        let path = std::path::PathBuf::from(file_path);
                        if let Some(parent) = path.parent() {
                            if let Err(e) = open::that(parent) {
                                eprintln!("Failed to open folder: {}", e);
                            }
                        }
                    } else {
                        // Fallback to opening the download location
                        let folder = std::path::PathBuf::from(&self.download_location);
                        if let Err(e) = open::that(&folder) {
                            eprintln!("Failed to open folder: {}", e);
                        }
                    }
                } else {
                    // Task not found, open default download location
                    let folder = std::path::PathBuf::from(&self.download_location);
                    if let Err(e) = open::that(&folder) {
                        eprintln!("Failed to open folder: {}", e);
                    }
                }
                Command::none()
            }

            // View navigation
            Message::SwitchToMain => {
                self.current_view = View::Main;
                Command::none()
            }

            Message::SwitchToSettings => {
                self.current_view = View::Settings;
                Command::none()
            }

            // Settings
            Message::DownloadLocationChanged(location) => {
                self.download_location = location;
                Command::none()
            }

            Message::BrowseDownloadLocation => {
                // Open file dialog to select download location
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.download_location = path.to_string_lossy().to_string();
                }
                Command::none()
            }

            Message::MaxConcurrentChanged(value) => {
                self.max_concurrent = value;
                Command::none()
            }

            Message::SegmentsChanged(value) => {
                self.segments_per_download = value;
                Command::none()
            }

            Message::QualityChanged(quality) => {
                self.quality = match quality.as_str() {
                    "Best Available" => VideoQuality::Best,
                    "1080p" => VideoQuality::Specific("1080".to_string()),
                    "720p" => VideoQuality::Specific("720".to_string()),
                    "480p" => VideoQuality::Specific("480".to_string()),
                    "Worst Available" => VideoQuality::Worst,
                    _ => VideoQuality::Best,
                };
                Command::none()
            }

            Message::SaveSettings => {
                let settings = AppSettings {
                    download_location: PathBuf::from(&self.download_location),
                    segments: self.segments_per_download,
                    max_concurrent: self.max_concurrent,
                    quality: self.quality.clone(),
                    chunk_size: 8192,
                    retry_attempts: 3,
                    enable_resume: true,
                };

                // Save settings to database
                let db_manager = Arc::clone(&self.db_manager);
                Command::perform(
                    async move {
                        save_settings_to_db(&db_manager, &settings)
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok::<(), String>(())
                    },
                    |result: Result<(), String>| {
                        match result {
                            Ok(_) => Message::SwitchToMain,
                            Err(_e) => {
                                // Handle error (for now, switch to main view)
                                Message::SwitchToMain
                            }
                        }
                    },
                )
            }

            // System
            Message::Tick => {
                // Drain progress updates from backend and log them for debugging
                // Use a non-blocking try_lock so the GUI never blocks if the backend
                // is busy (e.g. during a long extraction step). If the lock is
                // unavailable, skip this tick and let the next tick try again.
                if let Ok(mut bridge) = self.backend.try_lock() {
                    let mut update_count = 0usize;
                    while let Some(update) = bridge.try_receive_progress() {
                        update_count += 1;
                        eprintln!(
                            "🔔 GUI received progress update #{}: {:?}",
                            update_count, update
                        );

                        match update {
                            ProgressUpdate::ExtractionComplete(video_info) => {
                                let video_info = *video_info;
                                // Handle extraction completion
                                let title = video_info.title.clone();
                                let url = video_info.url.clone();

                                // Create output path
                                let output_path = PathBuf::from(&self.download_location)
                                    .join(format!("{}.mp4", sanitize_filename(&title)));

                                // Start download UI entry will be added when backend returns task id
                                let task_id = Uuid::new_v4().to_string();
                                let task_ui = DownloadTaskUI {
                                    id: task_id.clone(),
                                    title: title.clone(),
                                    url,
                                    progress: 0.0,
                                    speed: 0.0,
                                    status: "Queued".to_string(),
                                    downloaded_mb: 0.0,
                                    total_mb: video_info.filesize.unwrap_or(0) as f64
                                        / (1024.0 * 1024.0),
                                    eta_seconds: None,
                                    file_path: None,
                                };

                                self.active_downloads.push(task_ui);
                                self.status_message =
                                    format!("Added to download queue: {}", video_info.title);
                                eprintln!("✅ Added UI task for extraction: {}", title);
                            }
                            ProgressUpdate::DownloadProgress {
                                task_id,
                                progress,
                                speed,
                                downloaded,
                                total,
                                eta_seconds,
                            } => {
                                if let Some(task) =
                                    self.active_downloads.iter_mut().find(|t| t.id == task_id)
                                {
                                    task.progress = progress;
                                    task.speed = speed;
                                    task.downloaded_mb = downloaded as f64 / (1024.0 * 1024.0);
                                    task.total_mb = total as f64 / (1024.0 * 1024.0);
                                    task.eta_seconds = eta_seconds;

                                    eprintln!(
                                        "✅ Updated task {}: {:.1}% @ {:.2} MB/s",
                                        task.title,
                                        progress * 100.0,
                                        speed / 1024.0 / 1024.0
                                    );
                                } else {
                                    eprintln!("⚠️ Task {} not found in active_downloads!", task_id);
                                }
                            }
                            ProgressUpdate::DownloadComplete { task_id, file_path } => {
                                if let Some(task) =
                                    self.active_downloads.iter_mut().find(|t| t.id == task_id)
                                {
                                    task.status = "Completed".to_string();
                                    task.progress = 1.0;
                                    task.file_path = Some(file_path.clone());
                                }
                                self.status_message = "Download completed".to_string();
                                eprintln!("✅ [GUI] Download complete signal received for task {} at path: {}", task_id, file_path);
                            }
                            ProgressUpdate::DownloadFailed { task_id, error } => {
                                if let Some(task) =
                                    self.active_downloads.iter_mut().find(|t| t.id == task_id)
                                {
                                    task.status = "Failed".to_string();
                                }
                                self.status_message = format!("Download failed: {}", error);
                                eprintln!("❌ Download failed for {}: {}", task_id, error);
                            }
                            ProgressUpdate::TaskStatusChanged {
                                task_id,
                                status,
                                file_path,
                            } => {
                                if let Some(task) =
                                    self.active_downloads.iter_mut().find(|t| t.id == task_id)
                                {
                                    task.status = match status {
                                        TaskStatus::Queued => "Queued".to_string(),
                                        TaskStatus::Downloading => "Downloading".to_string(),
                                        TaskStatus::Paused => "Paused".to_string(),
                                        TaskStatus::Completed => "Completed".to_string(),
                                        TaskStatus::Failed(_) => "Failed".to_string(),
                                        TaskStatus::Cancelled => "Cancelled".to_string(),
                                    };
                                    if let Some(path) = file_path {
                                        task.file_path = Some(path);
                                    }
                                    eprintln!(
                                        "ℹ️ Task {} status changed -> {}",
                                        task.title, task.status
                                    );
                                } else {
                                    eprintln!("⚠️ Task {} not found for status change", task_id);
                                }
                            }
                        }
                    }

                    if update_count > 0 {
                        eprintln!("📊 Processed {} progress updates this tick", update_count);
                    }
                } else {
                    // Backend is currently locked by a background operation (e.g. extraction).
                    // Don't block the GUI; try again on the next tick.
                    // For debugging, print a low-verbosity note.
                    // eprintln!("🔕 Backend busy; skipping progress drain this tick");
                }

                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        use crate::gui::theme;
        use iced::widget::{button, column, container, row, text, Space};
        use iced::Length;

        // Sidebar
        let sidebar = container(
            column![
                // App Title / Logo Area
                container(text("Rustloader").size(24).style(theme::TEXT_PRIMARY)).padding(20),
                Space::with_height(20),
                // Navigation Items
                button(text("Downloads").size(16))
                    .style(iced::theme::Button::Custom(Box::new(
                        if self.current_view == View::Main {
                            theme::SidebarButtonStyle::Active
                        } else {
                            theme::SidebarButtonStyle::Inactive
                        }
                    )))
                    .width(Length::Fill)
                    .padding(12)
                    .on_press(Message::SwitchToMain),
                button(text("Settings").size(16))
                    .style(iced::theme::Button::Custom(Box::new(
                        if self.current_view == View::Settings {
                            theme::SidebarButtonStyle::Active
                        } else {
                            theme::SidebarButtonStyle::Inactive
                        }
                    )))
                    .width(Length::Fill)
                    .padding(12)
                    .on_press(Message::SwitchToSettings),
            ]
            .spacing(10)
            .padding(10),
        )
        .width(Length::Fixed(250.0))
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(
            theme::SidebarContainer,
        )));

        // Main Content Area
        let content = match self.current_view {
            View::Main => {
                use crate::gui::views::main_view;
                let quality_str = match self.quality {
                    VideoQuality::Best => "Best Available",
                    VideoQuality::Worst => "Worst Available",
                    VideoQuality::Specific(_) => "Custom",
                };
                main_view(
                    &self.url_input,
                    &self.active_downloads,
                    &self.status_message,
                    self.is_extracting,
                    self.url_error.as_deref(),
                    quality_str,
                    self.segments_per_download,
                )
            }
            View::Settings => {
                use crate::gui::views::settings_view;
                settings_view(
                    &self.download_location,
                    self.max_concurrent,
                    self.segments_per_download,
                )
            }
        };

        // Combine Sidebar and Content
        let main_layout = row![
            sidebar,
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20)
        ];

        // Wrap in Gradient Container
        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(
                theme::MainGradientContainer,
            )))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick)
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }
}

/// Sanitize filename for filesystem
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Load settings from database
async fn load_settings_from_db(db_manager: &DatabaseManager) -> Result<AppSettings> {
    let mut settings = AppSettings::default();

    // Load download location
    if let Some(location) = db_manager.get_setting("download_location").await? {
        settings.download_location = PathBuf::from(location);
    }

    // Load max concurrent
    if let Some(value) = db_manager.get_setting("max_concurrent").await? {
        if let Ok(val) = value.parse::<usize>() {
            settings.max_concurrent = val;
        }
    }

    // Load segments
    if let Some(value) = db_manager.get_setting("segments").await? {
        if let Ok(val) = value.parse::<usize>() {
            settings.segments = val;
        }
    }

    // Load quality
    if let Some(value) = db_manager.get_setting("quality").await? {
        settings.quality = match value.as_str() {
            "Best" => VideoQuality::Best,
            "Worst" => VideoQuality::Worst,
            _ => VideoQuality::Best,
        };
    }

    Ok(settings)
}

/// Save settings to database
async fn save_settings_to_db(db_manager: &DatabaseManager, settings: &AppSettings) -> Result<()> {
    db_manager
        .save_setting(
            "download_location",
            &settings.download_location.to_string_lossy(),
        )
        .await?;
    db_manager
        .save_setting("max_concurrent", &settings.max_concurrent.to_string())
        .await?;
    db_manager
        .save_setting("segments", &settings.segments.to_string())
        .await?;

    let quality_str = match settings.quality {
        VideoQuality::Best => "Best",
        VideoQuality::Worst => "Worst",
        VideoQuality::Specific(_) => "Custom",
    };
    db_manager.save_setting("quality", quality_str).await?;

    Ok(())
}

/// Convert technical error messages to user-friendly text
fn make_error_user_friendly(error: &str) -> String {
    let error_lower = error.to_lowercase();

    if error_lower.contains("truncated") || error_lower.contains("incomplete") {
        "Please enter a complete and valid URL".to_string()
    } else if error_lower.contains("invalid url") || error_lower.contains("malformed") {
        "This doesn't appear to be a valid video URL".to_string()
    } else if error_lower.contains("network")
        || error_lower.contains("connection")
        || error_lower.contains("timeout")
    {
        "Unable to connect. Please check your internet connection".to_string()
    } else if error_lower.contains("unavailable")
        || error_lower.contains("not found")
        || error_lower.contains("removed")
    {
        "This video is not available or has been removed".to_string()
    } else if error_lower.contains("private") || error_lower.contains("restricted") {
        "This video is private or restricted".to_string()
    } else if error_lower.contains("age") && error_lower.contains("restricted") {
        "This video is age-restricted and cannot be downloaded".to_string()
    } else if error_lower.contains("geo") || error_lower.contains("region") {
        "This video is not available in your region".to_string()
    } else if error_lower.contains("copyright") {
        "This video cannot be downloaded due to copyright restrictions".to_string()
    } else {
        // Generic fallback
        "Unable to process this URL. Please try a different video".to_string()
    }
}
