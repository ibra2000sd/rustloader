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
        /// Per-download "Ignore certificate errors (unsafe)" opt-in. Only the
        /// GUI checkbox sets this — bridge/extension requests never do.
        insecure_tls: bool,
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
    /// Tasks recovered from the event log at startup.
    ///
    /// The Downloads view builds its rows from `DownloadStarted`, which is
    /// emitted only for downloads started in the CURRENT session — so without
    /// this, a task paused before a restart came back into the queue with no
    /// row to represent it: invisible, and impossible to resume or cancel.
    TasksRestored {
        tasks: Vec<RestoredTask>,
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

/// One task rehydrated from the event log, in the shape the Downloads view
/// needs to render a row for it.
#[derive(Debug, Clone)]
pub struct RestoredTask {
    pub task_id: String,
    pub title: String,
    pub url: String,
    /// Same string vocabulary as `TaskStatusUpdated` ("Paused", "Queued", …),
    /// so a restored row and a live one are styled by identical logic.
    pub status: String,
}
