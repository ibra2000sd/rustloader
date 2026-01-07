use crate::extractor::VideoInfo;
use crate::gui::DownloadProgressData;
use std::path::PathBuf;

/// Commands sent from GUI to Backend
#[derive(Debug, Clone)]
pub enum BackendCommand {
    ExtractInfo {
        url: String,
    },
    StartDownload {
        video_info: VideoInfo,
        output_path: PathBuf,
        format_id: Option<String>,
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
