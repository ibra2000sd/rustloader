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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // DOWNLOAD PROGRESS CREATION AND INITIALIZATION
    // ============================================================

    #[test]
    fn test_download_progress_new() {
        let progress = DownloadProgress::new(1000, 16);

        assert_eq!(progress.total_bytes, 1000);
        assert_eq!(progress.downloaded_bytes, 0);
        assert_eq!(progress.speed, 0.0);
        assert_eq!(progress.eta, None);
        assert!(matches!(progress.status, DownloadStatus::Initializing));
        assert_eq!(progress.segments_completed, 0);
        assert_eq!(progress.total_segments, 16);
    }

    #[test]
    fn test_download_progress_new_zero_bytes() {
        let progress = DownloadProgress::new(0, 1);
        assert_eq!(progress.total_bytes, 0);
        assert_eq!(progress.downloaded_bytes, 0);
    }

    #[test]
    fn test_download_progress_new_large_file() {
        let progress = DownloadProgress::new(10_000_000_000, 32); // 10 GB
        assert_eq!(progress.total_bytes, 10_000_000_000);
        assert_eq!(progress.total_segments, 32);
    }

    // ============================================================
    // PROGRESS UPDATE TESTS
    // ============================================================

    #[test]
    fn test_progress_update_basic() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update(500, 100.0);

        assert_eq!(progress.downloaded_bytes, 500);
        assert_eq!(progress.speed, 100.0);
        assert!(progress.eta.is_some(), "ETA should be calculated");

        let eta = progress.eta.unwrap();
        assert_eq!(
            eta.as_secs(),
            5,
            "ETA should be 5 seconds (500 bytes remaining at 100 B/s)"
        );
    }

    #[test]
    fn test_progress_update_zero_speed() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update(100, 0.0);

        assert_eq!(progress.downloaded_bytes, 100);
        assert_eq!(progress.speed, 0.0);
        assert_eq!(progress.eta, None, "ETA should be None with zero speed");
    }

    #[test]
    fn test_progress_update_completed() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update(1000, 100.0);

        assert_eq!(progress.downloaded_bytes, 1000);
        assert!(progress.eta.is_some());
        assert_eq!(
            progress.eta.unwrap().as_secs(),
            0,
            "ETA should be 0 when complete"
        );
    }

    #[test]
    fn test_progress_update_over_total() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update(1500, 100.0); // More than total

        assert_eq!(progress.downloaded_bytes, 1500);
        // ETA calculation should handle this gracefully
    }

    #[test]
    fn test_progress_update_high_speed() {
        let mut progress = DownloadProgress::new(1_000_000, 10);
        progress.update(100_000, 10_000_000.0); // 10 MB/s

        assert_eq!(progress.speed, 10_000_000.0);
        let eta = progress.eta.unwrap();
        assert!(eta.as_secs() < 1, "ETA should be less than 1 second");
    }

    // ============================================================
    // SEGMENT PROGRESS TESTS
    // ============================================================

    #[test]
    fn test_update_segment_basic() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update_segment(5);

        assert_eq!(progress.segments_completed, 5);
        assert!(matches!(progress.status, DownloadStatus::Downloading));
    }

    #[test]
    fn test_update_segment_all_completed() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update_segment(10);

        assert_eq!(progress.segments_completed, 10);
        assert!(
            matches!(progress.status, DownloadStatus::Merging),
            "Status should be Merging when all segments complete"
        );
    }

    #[test]
    fn test_update_segment_transitions_from_initializing() {
        let mut progress = DownloadProgress::new(1000, 10);
        assert!(matches!(progress.status, DownloadStatus::Initializing));

        progress.update_segment(1);
        assert!(
            matches!(progress.status, DownloadStatus::Downloading),
            "Should transition from Initializing to Downloading"
        );
    }

    #[test]
    fn test_update_segment_beyond_total() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update_segment(15); // More than total

        assert_eq!(progress.segments_completed, 15);
        assert!(matches!(progress.status, DownloadStatus::Merging));
    }

    // ============================================================
    // STATUS TRANSITION TESTS
    // ============================================================

    #[test]
    fn test_complete_status() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.complete();

        assert!(matches!(progress.status, DownloadStatus::Completed));
        assert_eq!(
            progress.downloaded_bytes, progress.total_bytes,
            "Downloaded should equal total"
        );
        assert_eq!(
            progress.eta.unwrap().as_secs(),
            0,
            "ETA should be 0 when completed"
        );
    }

    #[test]
    fn test_failed_status() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.failed("Network timeout".to_string());

        match progress.status {
            DownloadStatus::Failed(msg) => {
                assert_eq!(msg, "Network timeout");
            }
            _ => panic!("Status should be Failed"),
        }
    }

    #[test]
    fn test_pause_status() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.pause();

        assert!(matches!(progress.status, DownloadStatus::Paused));
    }

    #[test]
    fn test_resume_status() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.pause();
        assert!(matches!(progress.status, DownloadStatus::Paused));

        progress.resume();
        assert!(matches!(progress.status, DownloadStatus::Downloading));
    }

    // ============================================================
    // PERCENTAGE CALCULATION TESTS
    // ============================================================

    #[test]
    fn test_percentage_zero_progress() {
        let progress = DownloadProgress::new(1000, 10);
        assert_eq!(progress.percentage(), 0.0);
    }

    #[test]
    fn test_percentage_half_complete() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update(500, 100.0);
        assert!((progress.percentage() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_percentage_complete() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.complete();
        assert_eq!(progress.percentage(), 1.0);
    }

    #[test]
    fn test_percentage_zero_total() {
        let progress = DownloadProgress::new(0, 1);
        assert_eq!(
            progress.percentage(),
            0.0,
            "Should return 0.0 for zero total, not panic"
        );
    }

    #[test]
    fn test_percentage_over_100() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update(1500, 100.0); // 150%

        let pct = progress.percentage();
        assert!(pct > 1.0, "Percentage can exceed 100% in edge cases");
    }

    #[test]
    fn test_percentage_precision() {
        let mut progress = DownloadProgress::new(12345, 10);
        progress.update(3333, 100.0);

        let pct = progress.percentage();
        let expected = 3333.0 / 12345.0; // ~0.26995
        assert!(
            (pct - expected).abs() < 0.00001,
            "Precision test: expected ~{}, got {}",
            expected,
            pct
        );
    }

    // ============================================================
    // DOWNLOAD STATUS TESTS
    // ============================================================

    #[test]
    fn test_status_equality() {
        let status1 = DownloadStatus::Downloading;
        let status2 = DownloadStatus::Downloading;
        assert_eq!(status1, status2);

        let status3 = DownloadStatus::Failed("Error".to_string());
        let status4 = DownloadStatus::Failed("Error".to_string());
        assert_eq!(status3, status4);
    }

    #[test]
    fn test_status_clone() {
        let status = DownloadStatus::Failed("Test error".to_string());
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_status_default() {
        let status = DownloadStatus::default();
        assert!(matches!(status, DownloadStatus::Initializing));
    }

    #[test]
    fn test_status_debug() {
        let status = DownloadStatus::Downloading;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Downloading"));
    }

    // ============================================================
    // ETA CALCULATION EDGE CASES
    // ============================================================

    #[test]
    fn test_eta_very_slow_speed() {
        let mut progress = DownloadProgress::new(1_000_000, 10);
        progress.update(100, 0.001); // Very slow: 0.001 bytes per second

        let eta = progress.eta.unwrap();
        assert!(
            eta.as_secs() > 900_000,
            "ETA should be very large for slow speed"
        );
    }

    #[test]
    fn test_eta_very_fast_speed() {
        let mut progress = DownloadProgress::new(1000, 10);
        progress.update(100, 1_000_000.0); // Very fast

        let eta = progress.eta.unwrap();
        assert!(eta.as_secs() < 1, "ETA should be very small for fast speed");
    }

    #[test]
    fn test_eta_updates_correctly() {
        let mut progress = DownloadProgress::new(1000, 10);

        progress.update(250, 100.0);
        let eta1 = progress.eta.unwrap();

        progress.update(500, 100.0);
        let eta2 = progress.eta.unwrap();

        progress.update(750, 100.0);
        let eta3 = progress.eta.unwrap();

        assert!(
            eta1 > eta2 && eta2 > eta3,
            "ETA should decrease as progress increases"
        );
    }

    // ============================================================
    // INTEGRATION TESTS
    // ============================================================

    #[test]
    fn test_full_download_lifecycle() {
        let mut progress = DownloadProgress::new(10_000, 8);

        // Start: Initializing
        assert!(matches!(progress.status, DownloadStatus::Initializing));

        // First segment downloads
        progress.update_segment(1);
        assert!(matches!(progress.status, DownloadStatus::Downloading));

        // Progress updates
        progress.update(2500, 1000.0);
        assert_eq!(progress.percentage(), 0.25);

        progress.update(5000, 1200.0);
        assert_eq!(progress.percentage(), 0.5);

        // All segments complete
        progress.update_segment(8);
        assert!(matches!(progress.status, DownloadStatus::Merging));

        // Download complete
        progress.complete();
        assert!(matches!(progress.status, DownloadStatus::Completed));
        assert_eq!(progress.percentage(), 1.0);
    }

    #[test]
    fn test_pause_resume_lifecycle() {
        let mut progress = DownloadProgress::new(10_000, 4);
        progress.update_segment(1);
        progress.update(2500, 1000.0);

        // Pause
        progress.pause();
        assert!(matches!(progress.status, DownloadStatus::Paused));
        let paused_bytes = progress.downloaded_bytes;

        // Resume
        progress.resume();
        assert!(matches!(progress.status, DownloadStatus::Downloading));
        assert_eq!(progress.downloaded_bytes, paused_bytes);

        // Continue download
        progress.update(5000, 1000.0);
        assert_eq!(progress.percentage(), 0.5);
    }

    #[test]
    fn test_failed_download_lifecycle() {
        let mut progress = DownloadProgress::new(10_000, 4);
        progress.update_segment(1);
        progress.update(2500, 1000.0);

        // Fail
        progress.failed("Connection lost".to_string());

        match progress.status {
            DownloadStatus::Failed(msg) => {
                assert_eq!(msg, "Connection lost");
            }
            _ => panic!("Should be Failed status"),
        }

        // Progress is retained even after failure
        assert_eq!(progress.downloaded_bytes, 2500);
    }
}
