//! Segment-based parallel downloading

use crate::downloader::progress::{DownloadProgress, DownloadStatus};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use reqwest::{Client, Response};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Segment information
#[derive(Debug, Clone)]
pub struct Segment {
    pub id: usize,
    pub start: u64,
    pub end: u64,
    pub size: u64,
    pub path: PathBuf,
}

/// Download a single segment
pub async fn download_segment(
    client: &Client,
    url: &str,
    segment: &Segment,
    progress_tx: mpsc::Sender<SegmentProgress>,
    retry_attempts: usize,
    retry_delay: Duration,
) -> Result<()> {
    let mut attempts = 0;

    while attempts <= retry_attempts {
        match download_segment_attempt(client, url, segment, &progress_tx).await {
            Ok(()) => return Ok(()),
            Err(e) if attempts < retry_attempts => {
                warn!("Segment {} download failed (attempt {}): {}", segment.id, attempts + 1, e);
                sleep(retry_delay).await;
                attempts += 1;
            }
            Err(e) => {
                error!("Segment {} download failed after {} attempts: {}", segment.id, retry_attempts + 1, e);
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Single attempt to download a segment
async fn download_segment_attempt(
    client: &Client,
    url: &str,
    segment: &Segment,
    progress_tx: &mpsc::Sender<SegmentProgress>,
) -> Result<()> {
    debug!("Downloading segment {} (bytes {}-{})", segment.id, segment.start, segment.end);

    // Create range header for this segment
    let range = if segment.start == segment.end {
        format!("bytes={}", segment.start)
    } else {
        format!("bytes={}-{}", segment.start, segment.end)
    };

    // Send request with range header
    let response = client
        .get(url)
        .header("Range", range)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    // Create file for this segment
    let mut file = File::create(&segment.path).await?;
    let mut downloaded = 0u64;
    let total_size = segment.end - segment.start + 1;

    // Track download speed
    let start_time = Instant::now();
    let mut last_update_time = start_time;
    let mut last_downloaded = 0u64;

    // Stream response to file
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;

        downloaded += chunk.len() as u64;

        // Update progress every second
        let now = Instant::now();
        if now.duration_since(last_update_time) >= Duration::from_secs(1) {
            let elapsed = now.duration_since(start_time).as_secs_f64();
            let speed = if elapsed > 0.0 { downloaded as f64 / elapsed } else { 0.0 };

            // Send progress update
            if let Err(e) = progress_tx.send(SegmentProgress {
                segment_id: segment.id,
                downloaded_bytes: downloaded,
                total_bytes: total_size,
                speed,
            }).await {
                warn!("Failed to send progress update for segment {}: {}", segment.id, e);
            }

            last_update_time = now;
            last_downloaded = downloaded;
        }
    }

    // Ensure file is flushed
    file.flush().await?;

    // Final progress update
    let elapsed = start_time.elapsed().as_secs_f64();
    let speed = if elapsed > 0.0 { downloaded as f64 / elapsed } else { 0.0 };

    if let Err(e) = progress_tx.send(SegmentProgress {
        segment_id: segment.id,
        downloaded_bytes: downloaded,
        total_bytes: total_size,
        speed,
    }).await {
        warn!("Failed to send final progress update for segment {}: {}", segment.id, e);
    }

    info!("Segment {} downloaded successfully ({} bytes)", segment.id, downloaded);

    Ok(())
}

/// Progress information for a segment
#[derive(Debug, Clone)]
pub struct SegmentProgress {
    pub segment_id: usize,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: f64, // bytes per second
}

/// Calculate optimal segments for a file
pub fn calculate_segments(file_size: u64, max_segments: usize) -> Vec<Segment> {
    if file_size == 0 {
        return Vec::new();
    }

    // Determine number of segments based on file size
    let segment_count = if file_size < 10 * 1024 * 1024 { // < 10MB
        1
    } else if file_size < 100 * 1024 * 1024 { // < 100MB
        std::cmp::min(4, max_segments)
    } else if file_size < 1024 * 1024 * 1024 { // < 1GB
        std::cmp::min(8, max_segments)
    } else {
        max_segments
    };

    let segment_size = file_size / segment_count as u64;
    let mut segments = Vec::with_capacity(segment_count);

    for i in 0..segment_count {
        let start = i as u64 * segment_size;
        let end = if i == segment_count - 1 {
            file_size - 1
        } else {
            (i + 1) as u64 * segment_size - 1
        };

        let size = end - start + 1;

        segments.push(Segment {
            id: i,
            start,
            end,
            size,
            path: PathBuf::from(format!("segment_{}.tmp", i)),
        });
    }

    segments
}
