//! Progress tracking for downloads

use std::time::Duration;

/// Progress tracking structure
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed: f64, // bytes per second
    pub eta: Option<Duration>,
    pub status: DownloadStatus,
    pub segments_completed: usize,
    pub total_segments: usize,
}

impl DownloadProgress {
    /// Create a new progress tracker
    pub fn new(total_bytes: u64, total_segments: usize) -> Self {
        Self {
            total_bytes,
            downloaded_bytes: 0,
            speed: 0.0,
            eta: None,
            status: DownloadStatus::Initializing,
            segments_completed: 0,
            total_segments,
        }
    }

    /// Update progress with new data
    pub fn update(&mut self, downloaded_bytes: u64, speed: f64) {
        self.downloaded_bytes = downloaded_bytes;
        self.speed = speed;

        // Calculate ETA if we have a speed
        if speed > 0.0 && self.downloaded_bytes < self.total_bytes {
            let remaining = self.total_bytes - self.downloaded_bytes;
            self.eta = Some(Duration::from_secs_f64((remaining as f64) / speed));
        } else if self.downloaded_bytes >= self.total_bytes {
            self.eta = Some(Duration::from_secs(0));
        } else {
            self.eta = None;
        }
    }

    /// Update segment progress
    pub fn update_segment(&mut self, completed: usize) {
        self.segments_completed = completed;

        // Update status based on segment progress
        if self.segments_completed >= self.total_segments {
            self.status = DownloadStatus::Merging;
        } else if self.status == DownloadStatus::Initializing {
            self.status = DownloadStatus::Downloading;
        }
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.status = DownloadStatus::Completed;
        self.downloaded_bytes = self.total_bytes;
        self.eta = Some(Duration::from_secs(0));
    }

    /// Mark as failed
    pub fn failed(&mut self, error: String) {
        self.status = DownloadStatus::Failed(error);
    }

    /// Mark as paused
    #[allow(dead_code)] // Reserved for pause/resume controls
    pub fn pause(&mut self) {
        self.status = DownloadStatus::Paused;
    }

    /// Resume from paused state
    #[allow(dead_code)] // Reserved for pause/resume controls
    pub fn resume(&mut self) {
        self.status = DownloadStatus::Downloading;
    }

    /// Get progress percentage (0.0 to 1.0)
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.downloaded_bytes as f64 / self.total_bytes as f64
    }
}

/// Download status
#[allow(dead_code)] // Additional states kept for GUI/queue status mapping
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DownloadStatus {
    #[default]
    Initializing,
    Downloading,
    Merging,
    Completed,
    Failed(String),
    Paused,
}
