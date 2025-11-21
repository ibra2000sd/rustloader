//! High-performance multi-threaded download engine

use crate::downloader::merger::{cleanup_segments, merge_segments, MergeProgress};
use crate::downloader::progress::{DownloadProgress, DownloadStatus};
use crate::downloader::segment::{calculate_segments, download_segment, SegmentProgress};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use reqwest::{Client, Response};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Download configuration
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub segments: usize,           // Number of parallel segments (default: 16)
    pub connections_per_segment: usize, // Connections per segment (default: 1)
    pub chunk_size: usize,         // Chunk size for streaming (default: 8192)
    pub retry_attempts: usize,     // Retry attempts per segment (default: 3)
    pub retry_delay: Duration,     // Delay between retries
    pub enable_resume: bool,       // Enable resume capability
    pub request_delay: Duration,   // Delay between segment requests
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            segments: 16,
            connections_per_segment: 1,
            chunk_size: 8192,
            retry_attempts: 3,
            retry_delay: Duration::from_secs(2),
            enable_resume: true,
            request_delay: Duration::from_millis(100),
        }
    }
}

/// High-performance multi-threaded download engine
pub struct DownloadEngine {
    client: Client,
    config: DownloadConfig,
}

impl DownloadEngine {
    /// Create new download engine with configuration
    pub fn new(config: DownloadConfig) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Download file with progress tracking
    pub async fn download(
        &self,
        url: &str,
        output_path: &Path,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        // Check if server supports range requests and get file size
        let (supports_ranges, file_size) = tokio::try_join!(
            self.supports_ranges(url),
            self.get_file_size(url)
        )?;

        // Initialize progress
        let mut progress = DownloadProgress::new(file_size, self.config.segments);

        // Send initial progress
        if let Err(e) = progress_tx.send(progress.clone()).await {
            warn!("Failed to send initial progress: {}", e);
        }

        // Determine download strategy
        if !supports_ranges || file_size < 1024 * 1024 { // < 1MB or no range support
            return self.download_simple(url, output_path, progress_tx).await;
        }

        // Calculate segments
        let segments = calculate_segments(file_size, self.config.segments);
        progress.total_segments = segments.len();

        // Create channels for segment progress
        let (segment_progress_tx, mut segment_progress_rx) = mpsc::channel::<SegmentProgress>(100);

        // Track segment completion
        let segment_progress = Arc::new(Mutex::new(vec![0u64; segments.len()]));

        // Clone for task closures
        let client = self.client.clone();
        let retry_attempts = self.config.retry_attempts;
        let retry_delay = self.config.retry_delay;
        let request_delay = self.config.request_delay;
        let segment_progress_clone = Arc::clone(&segment_progress);

        // Download all segments in parallel
        let download_tasks = stream::iter(segments.iter().enumerate())
            .map(|(i, segment)| {
                let client = client.clone();
                let segment = segment.clone();
                let segment_progress_tx = segment_progress_tx.clone();
                let segment_progress = Arc::clone(&segment_progress_clone);

                async move {
                    // Add delay between segment requests to avoid server throttling
                    if i > 0 {
                        sleep(request_delay).await;
                    }

                    // Download segment
                    let result = download_segment(
                        &client,
                        url,
                        &segment,
                        segment_progress_tx,
                        retry_attempts,
                        retry_delay,
                    ).await;

                    // Update segment progress
                    if result.is_ok() {
                        let mut progress = segment_progress.lock().await;
                        progress[i] = segment.size;
                    }

                    (i, result)
                }
            })
            .buffer_unordered(self.config.segments);

        // Process segment downloads
        let mut completed_segments = 0;
        let mut download_error = None;

        // Create a task to handle segment progress updates
        let progress_tx_clone = progress_tx.clone();
        let segment_progress_task = tokio::spawn(async move {
            let mut total_downloaded = 0u64;
            let mut last_update_time = std::time::Instant::now();
            let mut last_downloaded = 0u64;

            while let Some(segment_progress) = segment_progress_rx.recv().await {
                // Update segment progress
                let mut progress_vec = segment_progress.lock().await;
                progress_vec[segment_progress.segment_id] = segment_progress.downloaded_bytes;

                // Calculate total progress
                total_downloaded = progress_vec.iter().sum();

                // Update overall progress every second
                let now = std::time::Instant::now();
                if now.duration_since(last_update_time) >= Duration::from_secs(1) {
                    let elapsed = now.duration_since(last_update_time).as_secs_f64();
                    let speed = if elapsed > 0.0 { 
                        (total_downloaded - last_downloaded) as f64 / elapsed 
                    } else { 
                        0.0 
                    };

                    // Send progress update
                    let mut progress = DownloadProgress::new(file_size, segments.len());
                    progress.downloaded_bytes = total_downloaded;
                    progress.speed = speed;
                    progress.status = DownloadStatus::Downloading;

                    if let Err(e) = progress_tx_clone.send(progress).await {
                        warn!("Failed to send progress update: {}", e);
                        break;
                    }

                    last_update_time = now;
                    last_downloaded = total_downloaded;
                }
            }
        });

        // Wait for all segments to complete
        tokio::pin!(download_tasks);

        while let Some((segment_id, result)) = download_tasks.next().await {
            match result {
                Ok(()) => {
                    completed_segments += 1;
                    debug!("Segment {} completed", segment_id);
                }
                Err(e) => {
                    error!("Segment {} failed: {}", segment_id, e);
                    download_error = Some(e);
                    break;
                }
            }
        }

        // Abort the progress task
        segment_progress_task.abort();

        // Check if download failed
        if let Some(error) = download_error {
            let mut failed_progress = progress.clone();
            failed_progress.failed(error.to_string());

            if let Err(e) = progress_tx.send(failed_progress).await {
                warn!("Failed to send failed progress: {}", e);
            }

            return Err(error);
        }

        // Update progress to merging state
        progress.status = DownloadStatus::Merging;
        if let Err(e) = progress_tx.send(progress.clone()).await {
            warn!("Failed to send merging progress: {}", e);
        }

        // Create channel for merge progress
        let (merge_progress_tx, mut merge_progress_rx) = mpsc::channel::<MergeProgress>(10);

        // Spawn merge task
        let segments_paths: Vec<PathBuf> = segments.iter().map(|s| s.path.clone()).collect();
        let output_path_clone = output_path.to_path_buf();
        let merge_task = tokio::spawn(async move {
            merge_segments(&segments_paths, &output_path_clone, Some(merge_progress_tx)).await
        });

        // Process merge progress
        while let Some(merge_progress) = merge_progress_rx.recv().await {
            // Update progress
            progress.downloaded_bytes = merge_progress.total_bytes;
            progress.segments_completed = merge_progress.segment_index + 1;

            if let Err(e) = progress_tx.send(progress.clone()).await {
                warn!("Failed to send merge progress: {}", e);
                break;
            }
        }

        // Wait for merge to complete
        if let Err(e) = merge_task.await? {
            error!("Merge failed: {}", e);

            let mut failed_progress = progress.clone();
            failed_progress.failed(e.to_string());

            if let Err(e) = progress_tx.send(failed_progress).await {
                warn!("Failed to send merge failed progress: {}", e);
            }

            return Err(e);
        }

        // Clean up segment files
        if let Err(e) = cleanup_segments(&segments_paths).await {
            warn!("Failed to clean up segments: {}", e);
        }

        // Mark as completed
        progress.complete();
        if let Err(e) = progress_tx.send(progress).await {
            warn!("Failed to send completed progress: {}", e);
        }

        Ok(())
    }

    /// Simple download without segments (for servers that don't support range requests)
    async fn download_simple(
        &self,
        url: &str,
        output_path: &Path,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        debug!("Using simple download for URL: {}", url);

        // Send request
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        // Get file size
        let total_size = response.content_length().unwrap_or(0);

        // Initialize progress
        let mut progress = DownloadProgress::new(total_size, 1);
        progress.status = DownloadStatus::Downloading;

        // Send initial progress
        if let Err(e) = progress_tx.send(progress.clone()).await {
            warn!("Failed to send initial progress: {}", e);
        }

        // Create output file
        let mut file = File::create(output_path).await?;
        let mut downloaded = 0u64;

        // Track download speed
        let start_time = std::time::Instant::now();
        let mut last_update_time = start_time;
        let mut last_downloaded = 0u64;
        
        // Stream response to file
        let mut stream = response.bytes_stream();
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            
            downloaded += chunk.len() as u64;
            
            // Update progress every second
            let now = std::time::Instant::now();
            if now.duration_since(last_update_time) >= Duration::from_secs(1) {
                let elapsed = now.duration_since(start_time).as_secs_f64();
                let speed = if elapsed > 0.0 { downloaded as f64 / elapsed } else { 0.0 };
                
                // Update progress
                progress.downloaded_bytes = downloaded;
                progress.speed = speed;
                
                if let Err(e) = progress_tx.send(progress.clone()).await {
                    warn!("Failed to send progress update: {}", e);
                    break;
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
        
        progress.downloaded_bytes = downloaded;
        progress.speed = speed;
        progress.complete();
        
        if let Err(e) = progress_tx.send(progress).await {
            warn!("Failed to send final progress: {}", e);
        }
        
        Ok(())
    }
    
    /// Check if server supports range requests
    async fn supports_ranges(&self, url: &str) -> Result<bool> {
        let response = self.client.head(url).send().await?;
        
        let accepts_ranges = response
            .headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase() == "bytes")
            .unwrap_or(false);
            
        debug!("Server supports range requests: {}", accepts_ranges);
        Ok(accepts_ranges)
    }
    
    /// Get total file size
    async fn get_file_size(&self, url: &str) -> Result<u64> {
        let response = self.client.head(url).send().await?;
        
        let size = response
            .content_length()
            .ok_or_else(|| anyhow::anyhow!("Unknown file size"))?;
            
        debug!("File size: {} bytes", size);
        Ok(size)
    }
} = start_time;
        let mut last_downloaded = 0u64;

        // Stream response to file
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;

            downloaded += chunk.len() as u64;

            // Update progress every second
            let now = std::time::Instant::now();
            if now.duration_since(last_update_time) >= Duration::from_secs(1) {
                let elapsed = now.duration_since(start_time).as_secs_f64();
                let speed = if elapsed > 0.0 { downloaded as f64 / elapsed } else { 0.0 };

                // Update progress
                progress.downloaded_bytes = downloaded;
                progress.speed = speed;

                if let Err(e) = progress_tx.send(progress.clone()).await {
                    warn!("Failed to send progress update: {}", e);
                    break;
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

        progress.downloaded_bytes = downloaded;
        progress.speed = speed;
        progress.complete();

        if let Err(e) = progress_tx.send(progress).await {
            warn!("Failed to send final progress: {}", e);
        }

        info!("Simple download completed: {} bytes", downloaded);

        Ok(())
    }

    /// Check if server supports range requests
    async fn supports_ranges(&self, url: &str) -> Result<bool> {
        let response = self.client
            .head(url)
            .send()
            .await?;

        let accept_ranges = response.headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("none");

        Ok(accept_ranges.to_lowercase() == "bytes")
    }

    /// Get total file size
    async fn get_file_size(&self, url: &str) -> Result<u64> {
        let response = self.client
            .head(url)
            .send()
            .await?;

        Ok(response.content_length().unwrap_or(0))
    }
}

impl Default for DownloadEngine {
    fn default() -> Self {
        Self::new(DownloadConfig::default())
    }
}
