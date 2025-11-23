//! High-performance multi-threaded download engine

use crate::downloader::merger::{cleanup_segments, merge_segments, MergeProgress};
// progress types already imported above
use crate::downloader::segment::{calculate_segments, download_segment, SegmentProgress};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use reqwest::{Client, Response};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use crate::downloader::progress::{DownloadProgress, DownloadStatus};

fn parse_yt_dlp_progress(line: &str) -> Option<(f64, f64)> {
    // crude parser: find numeric percentage then optional speed
    if !line.contains('%') {
        return None;
    }
    let pct_pos = line.find('%')?;
    let before = &line[..pct_pos];
    let mut num_start = before.len();
    for (i, c) in before.chars().rev().enumerate() {
        if c.is_digit(10) || c == '.' {
            num_start = before.len() - i - 1;
        } else if num_start != before.len() {
            break;
        }
    }
    if num_start >= before.len() { return None; }
    let num_str = &before[num_start..].trim();
    let pct = num_str.parse::<f64>().ok()?;

    // parse speed (look for ' at ' and '/s')
    let mut speed_bps = 0.0;
    if let Some(at_idx) = line.find(" at ") {
        let after = &line[at_idx + 4..];
        if let Some(slash_idx) = after.find("/s") {
            let token = &after[..slash_idx].trim();
            let mut idx = 0;
            for (i, ch) in token.chars().enumerate() {
                if ch.is_digit(10) || ch == '.' { idx = i + 1; } else { break; }
            }
            if idx > 0 {
                if let Ok(num) = token[..idx].parse::<f64>() {
                    let unit = token[idx..].trim();
                    speed_bps = match unit {
                        "KiB" => num * 1024.0,
                        "MiB" => num * 1024.0 * 1024.0,
                        "GiB" => num * 1024.0 * 1024.0 * 1024.0,
                        "B" | "" => num,
                        _ => num,
                    };
                }
            }
        }
    }

    Some((pct, speed_bps))
}

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
        eprintln!("🚀🚀🚀 [ENGINE-ENTRY] download() ENTERED - First line executed!");
        eprintln!("    URL: {}", url);
        eprintln!("    Output: {:?}", output_path);

        // Send a conservative initial progress so GUI can show a task entry immediately
        eprintln!("📤 [ENGINE] Sending initial progress (Initializing)...");
        let mut initial = DownloadProgress::new(0, 1);
        initial.status = DownloadStatus::Initializing;
        if let Err(e) = progress_tx.send(initial.clone()).await {
            eprintln!("⚠️ [ENGINE] Failed to send initial progress: {}", e);
            warn!("Failed to send initial progress: {}", e);
        } else {
            eprintln!("✅ [ENGINE] Initial progress sent");
        }
        // Quick HLS detection: if URL looks like a playlist, fallback to yt-dlp
        if url.contains(".m3u8") || url.contains("/manifest") || url.contains("playlist") {
            eprintln!("🔀 [ENGINE] Taking path: yt-dlp fallback (HLS/playlist detected)");
            debug!("Detected HLS/playlist URL, using yt-dlp fallback: {}", url);
            return self.download_via_ytdlp(url, output_path, progress_tx).await;
        }
        // Check if server supports range requests and get file size.
        // If probing fails (some servers return unexpected responses or redirect to manifests),
        // fall back to yt-dlp to handle complex cases (HLS, DASH, etc.).
        eprintln!("🔍 [ENGINE] Probing server for range support and file size...");
        let supports_ranges_res = self.supports_ranges(url).await;
        eprintln!("✅ [ENGINE] supports_ranges() await returned: {:?}", &supports_ranges_res);
        let file_size_res = self.get_file_size(url).await;
        eprintln!("✅ [ENGINE] get_file_size() await returned: {:?}", &file_size_res);

        let (supports_ranges, file_size) = match (supports_ranges_res, file_size_res) {
            (Ok(r), Ok(s)) => {
                eprintln!("   - supports_ranges={}, file_size={}", r, s);
                (r, s)
            }
            (err1, err2) => {
                eprintln!("🔀 [ENGINE] Taking path: yt-dlp fallback (probing failed)");
                eprintln!("⚠️ [ENGINE] Probing ranges/size failed, falling back to yt-dlp. range_err={:?} size_err={:?}", err1.as_ref().err(), err2.as_ref().err());
                debug!("Probing ranges/size failed, falling back to yt-dlp. range_err={:?} size_err={:?}", err1.as_ref().err(), err2.as_ref().err());
                return self.download_via_ytdlp(url, output_path, progress_tx).await;
            }
        };

        // Initialize progress
        eprintln!("📊 [ENGINE] Initializing progress with file_size={} and segments={}", file_size, self.config.segments);
        let mut progress = DownloadProgress::new(file_size, self.config.segments);

        // Send initial progress
        eprintln!("📤 [ENGINE] Sending initial progress (based on probed file size)...");
        if let Err(e) = progress_tx.send(progress.clone()).await {
            eprintln!("⚠️ [ENGINE] Failed to send initial progress (probed): {}", e);
            warn!("Failed to send initial progress: {}", e);
        }

        // Determine download strategy
        if !supports_ranges || file_size < 1024 * 1024 { // < 1MB or no range support
            eprintln!("🔀 [ENGINE] Taking path: simple download (no ranges or small file). supports_ranges={}, file_size={}", supports_ranges, file_size);
            eprintln!("📥 [ENGINE] Using simple download (no ranges or small file). supports_ranges={}, file_size={}", supports_ranges, file_size);
            return self.download_simple(url, output_path, progress_tx).await;
        }

        eprintln!("📦 [ENGINE] Using segmented download path (ranges supported and file large enough)");

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
        let segments_for_tasks = segments.clone();
        let download_tasks = stream::iter(segments_for_tasks.into_iter().enumerate())
            .map(|(i, segment)| {
                let client = client.clone();
                let segment_progress_tx = segment_progress_tx.clone();
                let segment_progress = Arc::clone(&segment_progress_clone);

                async move {
                    // Add delay between segment requests to avoid server throttling
                    if i > 0 {
                        sleep(request_delay).await;
                        eprintln!("✅ [ENGINE] Sleep before starting segment {} completed", i);
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

                    eprintln!("✅ [ENGINE] download_segment completed for segment {}: success={}", i, result.is_ok());

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
        let segment_progress_clone_for_task = Arc::clone(&segment_progress_clone);
        let segments_count = segments.len();
        let segment_progress_task = tokio::spawn(async move {
            let mut total_downloaded = 0u64;
            let mut last_update_time = std::time::Instant::now();
            let mut last_downloaded = 0u64;

            while let Some(segment_progress) = segment_progress_rx.recv().await {
                // Update segment progress
                let mut progress_vec = segment_progress_clone_for_task.lock().await;
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
                    let mut progress = DownloadProgress::new(file_size, segments_count);
                    progress.downloaded_bytes = total_downloaded;
                    progress.speed = speed;
                    progress.status = DownloadStatus::Downloading;

                    if let Err(e) = progress_tx_clone.send(progress).await {
                        eprintln!("⚠️ [ENGINE] Failed to send aggregated progress: {}", e);
                        warn!("Failed to send progress update: {}", e);
                        break;
                    } else {
                        eprintln!("✅ [ENGINE] Sent aggregated progress to progress_tx_clone");
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
        let segments_paths_clone = segments_paths.clone();
        let output_path_clone = output_path.to_path_buf();
        let merge_task = tokio::spawn(async move {
            merge_segments(&segments_paths_clone, &output_path_clone, Some(merge_progress_tx)).await
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

    /// Fallback downloader using yt-dlp for HLS / complex streams
    async fn download_via_ytdlp(
        &self,
        url: &str,
        output_path: &Path,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        eprintln!("🎬 [YT-DLP] download_via_ytdlp called");
        eprintln!("   - URL: {}", url);

        // Clone the sender before spawning any background tasks and
        // use a best-effort initial send so a dropped receiver doesn't fail us.
        let progress_tx_for_spawn = progress_tx.clone();

        eprintln!("📤 [YT-DLP] Sending initial 0% progress (best-effort)");
        let mut initial = DownloadProgress::new(0, 1);
        initial.status = DownloadStatus::Downloading;
        initial.downloaded_bytes = 0;
        initial.speed = 0.0;
        // Best-effort: don't propagate an error if receiver was dropped.
        let _ = progress_tx_for_spawn.send(initial.clone()).await;

        // Prepare command: yt-dlp -f best -o <output_path> <url>
        eprintln!("🔧 [YT-DLP] Spawning yt-dlp process...");
        let out = output_path.to_string_lossy().to_string();
        let mut cmd = AsyncCommand::new("yt-dlp");
        cmd.arg("-f").arg("best").arg("-o").arg(out).arg(url);
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::null());

        let mut child = cmd.spawn()?;

        // Read stderr for progress lines — spawn a task that holds a clone of the sender
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            // Clone the sender and move it into the spawned task so the sender
            // remains alive while we parse and forward stderr progress lines.
            let progress_for_reader = progress_tx_for_spawn.clone();

            tokio::spawn(async move {
                while let Some(Ok(line)) = lines.next_line().await.transpose() {
                    eprintln!("📄 [YT-DLP] stderr: {}", line);
                    // Try to parse percentage and speed from lines like:
                    // [download]  12.5% of ~10.50MiB at 1.23MiB/s ETA 00:07
                    if let Some((pct, speed_bps)) = parse_yt_dlp_progress(&line) {
                        eprintln!("🔁 [YT-DLP] Parsed progress: {}% speed={} B/s", pct, speed_bps);
                        let mut p = DownloadProgress::new(100, 1);
                        p.status = DownloadStatus::Downloading;
                        // represent percentage on a 0..100 scale when total size unknown
                        p.downloaded_bytes = pct.round() as u64;
                        p.speed = speed_bps;
                        p.segments_completed = 0;

                        // Best-effort send: don't treat a closed receiver as fatal here
                        if let Err(e) = progress_for_reader.send(p).await {
                            eprintln!("⚠️ [YT-DLP] Failed to send parsed progress: {}", e);
                            warn!("Failed to send parsed progress (yt-dlp): {}", e);
                            break;
                        }
                    }
                }
            });
        }

        // Wait for child to finish
        eprintln!("🔚 [YT-DLP] Waiting for yt-dlp process to exit...");
        let status = child.wait().await?;
        eprintln!("🔚 [YT-DLP] Process exited with: {:?}", status.code());
        if status.success() {
            eprintln!("✅ [YT-DLP] Download successful");
            let mut done = DownloadProgress::new(0, 1);
            done.status = DownloadStatus::Completed;
            done.downloaded_bytes = 0;
            done.speed = 0.0;
            done.complete();
            let _ = progress_tx.send(done).await;
            Ok(())
        } else {
            eprintln!("❌ [YT-DLP] Download failed");
            let mut failed = DownloadProgress::new(0, 1);
            failed.failed("yt-dlp failed".to_string());
            let _ = progress_tx.send(failed).await;
            Err(anyhow::anyhow!("yt-dlp download failed"))
        }
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
        // wrap HEAD in a timeout to avoid blocking indefinitely
        match tokio::time::timeout(std::time::Duration::from_secs(10), self.client.head(url).send()).await {
            Ok(Ok(response)) => {
                let accepts_ranges = response
                    .headers()
                    .get("accept-ranges")
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.to_lowercase() == "bytes")
                    .unwrap_or(false);
                debug!("Server supports range requests: {}", accepts_ranges);
                Ok(accepts_ranges)
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ [ENGINE] HEAD request failed: {}", e);
                Err(e.into())
            }
            Err(_) => {
                eprintln!("⏰ [ENGINE] HEAD request timeout (10s)");
                Err(anyhow::anyhow!("HEAD timeout"))
            }
        }
    }
    
    /// Get total file size
    async fn get_file_size(&self, url: &str) -> Result<u64> {
        // wrap HEAD in a timeout to avoid blocking indefinitely
        match tokio::time::timeout(std::time::Duration::from_secs(10), self.client.head(url).send()).await {
            Ok(Ok(response)) => {
                let size = response
                    .content_length()
                    .ok_or_else(|| anyhow::anyhow!("Unknown file size"))?;
                debug!("File size: {} bytes", size);
                Ok(size)
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ [ENGINE] HEAD request failed: {}", e);
                Err(e.into())
            }
            Err(_) => {
                eprintln!("⏰ [ENGINE] HEAD request timeout (10s)");
                Err(anyhow::anyhow!("HEAD timeout"))
            }
        }
    }
} 

impl Default for DownloadEngine {
    fn default() -> Self {
        Self::new(DownloadConfig::default())
    }
}
