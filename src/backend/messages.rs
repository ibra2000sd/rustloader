use crate::extractor::VideoInfo;
use crate::gui::DownloadProgressData;
use crate::utils::config::{OutputFormat, VideoQuality};
use std::path::PathBuf;

/// Commands sent from GUI to Backend
#[derive(Debug, Clone)]
pub enum BackendCommand {
    ExtractInfo {
        url: String,
        /// Per-request cookie override: a Netscape-format cookies.txt (the
        /// browser-bridge writes one from extension-supplied cookies,
        /// F-EXT-001). `None` = the extractor's settings-derived cookies.
        /// Applied via `CookieConfig { file }` (invariant I-7).
        cookies_file: Option<std::path::PathBuf>,
    },
    StartDownload {
        // Boxed: VideoInfo is large; boxing keeps the enum variants similar in
        // size (clippy::large_enum_variant).
        video_info: Box<VideoInfo>,
        output_path: PathBuf,
        format_id: Option<String>,
        /// The user's quality choice, applied by format selection when no
        /// explicit `format_id` is given (B-GUI-003).
        quality: VideoQuality,
        /// The user's output format/container choice (B-GUI-005). `Best`
        /// keeps today's behaviour; anything else routes the download through
        /// yt-dlp for a remux or audio extraction.
        output_format: OutputFormat,
    },
    PauseDownload(String),
    ResumeDownload(String),
    CancelDownload(String),
    RemoveTask(String),
    ClearCompleted,
    ResumeAll,
    // System
    Shutdown,
}

/// Events sent from Backend to GUI
#[derive(Debug, Clone)]
pub enum BackendEvent {
    // Extraction
    ExtractionStarted,
    ExtractionCompleted(Result<VideoInfo, String>),

    // Download Life-cycle
    DownloadStarted {
        task_id: String,
        video_info: VideoInfo,
    },
    DownloadProgress {
        task_id: String,
        data: DownloadProgressData,
    },
    DownloadCompleted {
        task_id: String,
        /// Final on-disk path after organization, so the GUI's Open File /
        /// Show in Folder actions point at the real file.
        file_path: Option<String>,
    },
    DownloadFailed {
        task_id: String,
        error: String,
    },

    // Task Status Updates (for Pause/Resume/Cancel confirmation)
    TaskStatusUpdated {
        task_id: String,
        status: String, // "Paused", "Cancelled", "Queued"
    },

    // System
    Error(String),
}
