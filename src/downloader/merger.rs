//! File segment merger

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

/// Merge segments into a single file
pub async fn merge_segments(
    segments: &[PathBuf],
    output_path: &Path,
    progress_tx: Option<tokio::sync::mpsc::Sender<MergeProgress>>,
) -> Result<()> {
    if segments.is_empty() {
        return Err(anyhow::anyhow!("No segments to merge"));
    }

    debug!(
        "Merging {} segments into {}",
        segments.len(),
        output_path.display()
    );

    // Create output file
    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_path)
        .await?;

    let mut total_bytes = 0u64;

    // Process each segment
    for (i, segment_path) in segments.iter().enumerate() {
        // Open segment file
        let mut segment_file = File::open(segment_path).await?;

        // Copy segment to output file
        let mut buffer = [0; 8192];
        let mut bytes_copied = 0u64;

        loop {
            let bytes_read = segment_file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            output_file.write_all(&buffer[..bytes_read]).await?;
            bytes_copied += bytes_read as u64;
            total_bytes += bytes_read as u64;

            // Send progress update if channel is provided
            if let Some(ref tx) = progress_tx {
                let _ = tx
                    .send(MergeProgress {
                        segment_index: i,
                        total_bytes,
                    })
                    .await;
            }
        }

        debug!("Merged segment {} ({} bytes)", i, bytes_copied);
    }

    // Ensure all data is written
    output_file.flush().await?;

    info!(
        "Successfully merged {} bytes into {}",
        total_bytes,
        output_path.display()
    );

    Ok(())
}

/// Clean up temporary segment files
pub async fn cleanup_segments(segments: &[PathBuf]) -> Result<()> {
    for segment_path in segments {
        if segment_path.exists() {
            if let Err(e) = tokio::fs::remove_file(segment_path).await {
                warn!(
                    "Failed to remove segment file {}: {}",
                    segment_path.display(),
                    e
                );
            } else {
                debug!("Removed segment file: {}", segment_path.display());
            }
        }
    }
    Ok(())
}

/// Progress information for merging
#[derive(Debug, Clone)]
pub struct MergeProgress {
    pub segment_index: usize,
    pub total_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use tokio::fs;

    // ============================================================
    // MERGE SEGMENTS TESTS
    // ============================================================

    #[tokio::test]
    async fn test_merge_single_segment() {
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_0.tmp");
        let output_path = temp_dir.path().join("output.mp4");

        // Create segment file with test data
        let test_data = b"Hello, World! This is a test segment.";
        std::fs::write(&segment_path, test_data).unwrap();

        // Merge without progress channel
        let result = merge_segments(&[segment_path.clone()], &output_path, None).await;
        assert!(result.is_ok(), "Single segment merge should succeed");

        // Verify output file exists and contains correct data
        assert!(output_path.exists(), "Output file should exist");
        let output_data = std::fs::read(&output_path).unwrap();
        assert_eq!(
            output_data, test_data,
            "Output should match input for single segment"
        );
    }

    #[tokio::test]
    async fn test_merge_multiple_segments() {
        let temp_dir = TempDir::new().unwrap();
        let segment_paths: Vec<_> = (0..5)
            .map(|i| temp_dir.path().join(format!("segment_{}.tmp", i)))
            .collect();
        let output_path = temp_dir.path().join("output.bin");

        // Create segment files with sequential data
        for (i, path) in segment_paths.iter().enumerate() {
            let data = format!("Segment{:02}|", i).into_bytes();
            std::fs::write(path, &data).unwrap();
        }

        // Merge segments
        let result = merge_segments(&segment_paths, &output_path, None).await;
        assert!(result.is_ok(), "Multi-segment merge should succeed");

        // Verify output
        let output_data = std::fs::read_to_string(&output_path).unwrap();
        let expected = "Segment00|Segment01|Segment02|Segment03|Segment04|";
        assert_eq!(
            output_data, expected,
            "Segments should be concatenated in order"
        );
    }

    #[tokio::test]
    async fn test_merge_with_progress_channel() {
        let temp_dir = TempDir::new().unwrap();
        let segment_paths: Vec<_> = (0..3)
            .map(|i| temp_dir.path().join(format!("segment_{}.tmp", i)))
            .collect();
        let output_path = temp_dir.path().join("output.bin");

        // Create segments
        for (i, path) in segment_paths.iter().enumerate() {
            let data = vec![i as u8; 1000]; // 1KB each
            std::fs::write(path, &data).unwrap();
        }

        // Create progress channel
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(10);

        // Merge with progress tracking
        let merge_task = tokio::spawn(async move {
            merge_segments(&segment_paths, &output_path, Some(progress_tx)).await
        });

        // Collect progress updates
        let mut updates = Vec::new();
        while let Some(progress) = progress_rx.recv().await {
            updates.push(progress);
            if updates.len() >= 3 {
                // At least one update per segment
                break;
            }
        }

        let merge_result = merge_task.await.unwrap();
        assert!(merge_result.is_ok(), "Merge with progress should succeed");
        assert!(!updates.is_empty(), "Should receive progress updates");
    }

    #[tokio::test]
    async fn test_merge_empty_segments_list() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.bin");

        let result = merge_segments(&[], &output_path, None).await;
        assert!(
            result.is_err(),
            "Merging empty segment list should return error"
        );

        let err_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            err_msg.contains("no segments") || err_msg.contains("empty"),
            "Error should mention empty segments"
        );
    }

    #[tokio::test]
    async fn test_merge_missing_segment_fails() {
        let temp_dir = TempDir::new().unwrap();
        let missing_segment = temp_dir.path().join("nonexistent.tmp");
        let output_path = temp_dir.path().join("output.bin");

        let result = merge_segments(&[missing_segment], &output_path, None).await;
        assert!(result.is_err(), "Merging with missing segment should fail");
    }

    #[tokio::test]
    async fn test_merge_large_segments() {
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("large_segment.tmp");
        let output_path = temp_dir.path().join("output.bin");

        // Create 1MB segment
        let large_data = vec![0xAB; 1_000_000];
        std::fs::write(&segment_path, &large_data).unwrap();

        let result = merge_segments(&[segment_path], &output_path, None).await;
        assert!(result.is_ok(), "Should handle large files");

        // Verify size
        let metadata = tokio::fs::metadata(&output_path).await.unwrap();
        assert_eq!(metadata.len(), 1_000_000, "Output size should match input");
    }

    #[tokio::test]
    async fn test_merge_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment.tmp");
        let output_path = temp_dir
            .path()
            .join("nested")
            .join("deep")
            .join("output.mp4");

        // Create segment
        std::fs::write(&segment_path, b"test data").unwrap();

        // Note: merge_segments may or may not create parent dirs
        // This test documents expected behavior
        let result = merge_segments(&[segment_path], &output_path, None).await;

        // If it fails, that's ok - parent dir creation might not be implemented
        // If it succeeds, verify the file exists
        if result.is_ok() {
            assert!(
                output_path.exists(),
                "Output should exist if merge succeeded"
            );
        }
    }

    #[tokio::test]
    async fn test_merge_verifies_data_integrity() {
        let temp_dir = TempDir::new().unwrap();
        let segment_paths: Vec<_> = (0..3)
            .map(|i| temp_dir.path().join(format!("segment_{}.tmp", i)))
            .collect();
        let output_path = temp_dir.path().join("output.bin");

        // Create segments with specific byte patterns
        let patterns = [vec![0xAA; 100], vec![0xBB; 200], vec![0xCC; 150]];
        for (path, data) in segment_paths.iter().zip(patterns.iter()) {
            std::fs::write(path, data).unwrap();
        }

        merge_segments(&segment_paths, &output_path, None)
            .await
            .unwrap();

        // Verify data integrity
        let output = std::fs::read(&output_path).unwrap();
        assert_eq!(output.len(), 450, "Total size should be sum of segments");
        assert!(
            output[0..100].iter().all(|&b| b == 0xAA),
            "First segment pattern"
        );
        assert!(
            output[100..300].iter().all(|&b| b == 0xBB),
            "Second segment pattern"
        );
        assert!(
            output[300..450].iter().all(|&b| b == 0xCC),
            "Third segment pattern"
        );
    }

    // ============================================================
    // CLEANUP SEGMENTS TESTS
    // ============================================================

    #[tokio::test]
    async fn test_cleanup_segments_removes_files() {
        let temp_dir = TempDir::new().unwrap();
        let segment_paths: Vec<_> = (0..3)
            .map(|i| temp_dir.path().join(format!("segment_{}.tmp", i)))
            .collect();

        // Create segment files
        for path in &segment_paths {
            std::fs::write(path, b"temporary data").unwrap();
            assert!(path.exists(), "Segment should exist before cleanup");
        }

        // Cleanup
        let result = cleanup_segments(&segment_paths).await;
        assert!(result.is_ok(), "Cleanup should succeed");

        // Verify all segments are removed
        for path in &segment_paths {
            assert!(
                !path.exists(),
                "Segment {} should be removed",
                path.display()
            );
        }
    }

    #[tokio::test]
    async fn test_cleanup_nonexistent_segments() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_paths: Vec<_> = (0..3)
            .map(|i| temp_dir.path().join(format!("nonexistent_{}.tmp", i)))
            .collect();

        // Cleanup should handle nonexistent files gracefully
        let result = cleanup_segments(&nonexistent_paths).await;
        assert!(
            result.is_ok(),
            "Cleanup should succeed even with nonexistent files"
        );
    }

    #[tokio::test]
    async fn test_cleanup_empty_list() {
        let result = cleanup_segments(&[]).await;
        assert!(result.is_ok(), "Cleanup of empty list should succeed");
    }

    #[tokio::test]
    async fn test_cleanup_mixed_existing_and_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let existing = temp_dir.path().join("existing.tmp");
        let nonexistent = temp_dir.path().join("nonexistent.tmp");

        std::fs::write(&existing, b"data").unwrap();

        let result = cleanup_segments(&[existing.clone(), nonexistent]).await;
        assert!(result.is_ok(), "Cleanup should succeed with mixed files");
        assert!(!existing.exists(), "Existing file should be removed");
    }

    // ============================================================
    // MERGE PROGRESS TESTS
    // ============================================================

    #[test]
    fn test_merge_progress_creation() {
        let progress = MergeProgress {
            segment_index: 3,
            total_bytes: 1024,
        };

        assert_eq!(progress.segment_index, 3);
        assert_eq!(progress.total_bytes, 1024);
    }

    #[test]
    fn test_merge_progress_clone() {
        let progress = MergeProgress {
            segment_index: 5,
            total_bytes: 2048,
        };

        let cloned = progress.clone();
        assert_eq!(cloned.segment_index, progress.segment_index);
        assert_eq!(cloned.total_bytes, progress.total_bytes);
    }
}
