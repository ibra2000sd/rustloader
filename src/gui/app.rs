//! Main GUI application
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::backend::{BackendActor, BackendCommand, BackendEvent};
use crate::database::{initialize_database, DatabaseManager};
use crate::extractor::VideoInfo;
use crate::gui::clipboard;
// DownloadProgressData defined below
use crate::queue::TaskStatus;
use crate::utils::config::{AppSettings, VideoQuality};

use anyhow::Result;
use iced::{executor, Application, Command, Element, Subscription, Theme};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tracing::error;
use uuid::Uuid;

/// Main application state
pub struct RustloaderApp {
    // Core components
    backend_sender: mpsc::Sender<BackendCommand>,
    backend_receiver: Arc<Mutex<Option<mpsc::Receiver<BackendEvent>>>>,
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

/// Progress data transfer object
#[derive(Debug, Clone)]
pub struct DownloadProgressData {
    pub progress: f32,
    pub speed: f64,
    pub downloaded: u64,
    pub total: u64,
    pub eta: Option<u64>,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    // Input events
    UrlInputChanged(String),
    DownloadButtonPressed,
    PasteFromClipboard,
    ClearUrlInput,

    // Backend Events
    BackendEventReceived(BackendEvent),

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

impl Application for RustloaderApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        // Initialize settings
        let mut settings = AppSettings::default();

        let db_path = crate::utils::get_database_path();
        let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

        // Create a single runtime and keep it alive for the app lifetime
        let rt = Runtime::new().expect("Failed to create tokio runtime");

        let db_pool = rt
            .block_on(initialize_database(&db_url))
            .expect("Failed to initialize database");

        let db_manager = Arc::new(DatabaseManager::new(db_pool));

        // Load settings from database
        if let Ok(loaded_settings) = rt.block_on(load_settings_from_db(&db_manager)) {
            settings = loaded_settings;
        }

        // Initialize Backend Actor
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);

        // Spawn the actor on the runtime
        let settings_clone = settings.clone();
        rt.spawn(async move {
            match BackendActor::new(settings_clone, cmd_rx, event_tx).await {
                Ok(actor) => actor.run().await,
                Err(e) => error!("Failed to start backend actor: {}", e),
            }
        });

        let app = Self {
            backend_sender: cmd_tx,
            backend_receiver: Arc::new(Mutex::new(Some(event_rx))),
            db_manager,
            runtime: Arc::new(rt),
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

                    // Send extract command to backend
                    let url = self.url_input.clone();
                    let _ = self.backend_sender.try_send(BackendCommand::ExtractInfo { url });
                }
                Command::none()
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
            
            // Backend Events Handling
            Message::BackendEventReceived(event) => {
                 match event {
                     BackendEvent::ExtractionStarted => {
                         self.is_extracting = true;
                         self.status_message = "Extracting video information...".to_string();
                     }
                     BackendEvent::ExtractionCompleted(result) => {
                         self.is_extracting = false;
                         match result {
                             Ok(video_info) => {
                                 // Auto-start download logic
                                 let output_path = PathBuf::from(&self.download_location)
                                    .join(format!("{}.mp4", sanitize_filename(&video_info.title)));
                                 
                                 self.url_input.clear();
                                 self.url_error = None;
                                 self.status_message = format!("Starting download: {}", video_info.title);

                                 // Send start command
                                 let _ = self.backend_sender.try_send(BackendCommand::StartDownload {
                                     video_info,
                                     output_path,
                                     format_id: None // Auto format
                                 });
                             }
                             Err(e) => {
                                 self.url_error = Some(make_error_user_friendly(&e));
                                 self.status_message = "Extraction failed".to_string();
                             }
                         }
                     }
                     BackendEvent::DownloadStarted { task_id, video_info } => {
                         let task_ui = DownloadTaskUI {
                             id: task_id.clone(),
                             title: video_info.title.clone(),
                             url: video_info.url.clone(),
                             progress: 0.0,
                             speed: 0.0,
                             status: "Queued".to_string(),
                             downloaded_mb: 0.0,
                             total_mb: video_info.filesize.unwrap_or(0) as f64 / (1024.0 * 1024.0),
                             eta_seconds: None,
                             file_path: None,
                         };
                         self.active_downloads.push(task_ui);
                         self.status_message = format!("Added to queue: {}", video_info.title);
                     }
                     BackendEvent::DownloadProgress { task_id, data } => {
                         if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                             task.progress = data.progress;
                             task.speed = data.speed;
                             task.downloaded_mb = data.downloaded as f64 / (1024.0 * 1024.0);
                             task.total_mb = data.total as f64 / (1024.0 * 1024.0);
                             task.eta_seconds = data.eta;
                         }
                     }
                     BackendEvent::DownloadCompleted { task_id } => {
                         if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                             task.status = "Completed".to_string();
                             task.progress = 1.0;
                         }
                         self.status_message = "Download completed".to_string();
                     }
                     BackendEvent::DownloadFailed { task_id, error } => {
                         if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                             task.status = "Failed".to_string();
                         }
                         self.status_message = format!("Failed: {}", error);
                     }
                     BackendEvent::TaskStatusUpdated { task_id, status } => {
                         if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == task_id) {
                             task.status = status;
                         }
                     }
                     BackendEvent::Error(e) => {
                         self.status_message = format!("Error: {}", e);
                     }
                    _ => {}
                 }
                 Command::none()
            }

            // Queue control
            Message::PauseDownload(task_id) => {
                let _ = self.backend_sender.try_send(BackendCommand::PauseDownload(task_id));
                Command::none()
            }

            Message::ResumeDownload(task_id) => {
                let _ = self.backend_sender.try_send(BackendCommand::ResumeDownload(task_id));
                Command::none()
            }

            Message::CancelDownload(task_id) => {
                // Optimistic UI update
                self.active_downloads.retain(|t| t.id != task_id);
                let _ = self.backend_sender.try_send(BackendCommand::CancelDownload(task_id));
                Command::none()
            }

            Message::RemoveCompleted(task_id) => {
                 self.active_downloads.retain(|t| t.id != task_id);
                 let _ = self.backend_sender.try_send(BackendCommand::RemoveTask(task_id));
                 Command::none()
            }

            Message::ClearAllCompleted => {
                self.active_downloads.retain(|t| t.status != "Completed");
                let _ = self.backend_sender.try_send(BackendCommand::ClearCompleted);
                Command::none()
            }

            Message::RetryDownload(task_id) => {
                // Retry logic needs full restart usually? 
                // For now, if we have URL, we could potential restart.
                // But simplified: just set status to Queued?
                // Or send Resume?
                let _ = self.backend_sender.try_send(BackendCommand::ResumeDownload(task_id));
                Command::none()
            }

            Message::OpenFile(task_id) => {
                if let Some(task) = self.active_downloads.iter().find(|t| t.id == task_id) {
                    // Logic to find file... 
                    // Previously we set file_path in DownloadCompleted.
                    // We might need BackendEvent to include path in DownloadCompleted?
                    // Or keep track of it.
                    // For now, construct from download location + title?
                     if let Some(file_path) = &task.file_path {
                        let path = std::path::PathBuf::from(file_path);
                        let _ = open::that(&path);
                    } else {
                         // Fallback guess
                         let path = PathBuf::from(&self.download_location)
                                    .join(format!("{}.mp4", sanitize_filename(&task.title)));
                         if path.exists() {
                             let _ = open::that(&path);
                         }
                    }
                }
                Command::none()
            }

            Message::OpenDownloadFolder(task_id) => {
                let folder = std::path::PathBuf::from(&self.download_location);
                let _ = open::that(&folder);
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
                        Message::SwitchToMain
                    },
                )
            }
            
            Message::Tick => Command::none(), // No polling needed
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // Create a backend subscription using the receiver in self.
        // We use a wrapper struct to handle the Hash identity for unfold.
        struct BackendListener(Arc<Mutex<Option<mpsc::Receiver<BackendEvent>>>>);
        
        impl std::hash::Hash for BackendListener {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                // Hash the pointer address of the Arc to ensure identity stability
                (Arc::as_ptr(&self.0) as usize).hash(state);
            }
        }

        // We use unfold. State::Starting(Listener), State::Ready(Rx), State::Empty
        enum State {
            Starting(BackendListener),
            Ready(mpsc::Receiver<BackendEvent>),
            Empty,
        }

        let listener = BackendListener(self.backend_receiver.clone());

        iced::subscription::unfold(
            "backend-listener",
            State::Starting(listener),
            |state| async move {
                match state {
                    State::Starting(wrapper) => {
                        let rx_opt = wrapper.0.lock().unwrap().take();
                        if let Some(rx) = rx_opt {
                             let mut rx = rx;
                             match rx.recv().await {
                                 Some(event) => (Message::BackendEventReceived(event), State::Ready(rx)),
                                 None => std::future::pending().await
                             }
                        } else {
                            std::future::pending().await
                        }
                    },
                    State::Ready(mut rx) => {
                        match rx.recv().await {
                            Some(event) => (Message::BackendEventReceived(event), State::Ready(rx)),
                            None => std::future::pending().await
                        }
                    },
                    State::Empty => {
                         std::future::pending().await
                    }
                }
            }
        )
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
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
