//! File segment merger

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter};
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

    debug!("Merging {} segments into {}", segments.len(), output_path.display());

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
        let segment_size = segment_file.metadata().await?.len();

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
                let _ = tx.send(MergeProgress {
                    segment_index: i,
                    segment_progress: bytes_copied as f64 / segment_size as f64,
                    total_bytes,
                }).await;
            }
        }

        debug!("Merged segment {} ({} bytes)", i, bytes_copied);
    }

    // Ensure all data is written
    output_file.flush().await?;

    info!("Successfully merged {} bytes into {}", total_bytes, output_path.display());

    Ok(())
}

/// Clean up temporary segment files
pub async fn cleanup_segments(segments: &[PathBuf]) -> Result<()> {
    for segment_path in segments {
        if segment_path.exists() {
            if let Err(e) = tokio::fs::remove_file(segment_path).await {
                warn!("Failed to remove segment file {}: {}", segment_path.display(), e);
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
    pub segment_progress: f64, // 0.0 to 1.0
    pub total_bytes: u64,
}
