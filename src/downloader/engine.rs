//! High-performance multi-threaded download engine
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_assignments
)]

use crate::downloader::merger::{cleanup_segments, merge_segments, MergeProgress};
// progress types already imported above
use crate::downloader::progress::{DownloadProgress, DownloadStatus};
use crate::downloader::segment::{calculate_segments, download_segment, SegmentProgress};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as AsyncCommand;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::{debug, error, warn};

fn parse_yt_dlp_progress(line: &str) -> Option<(f64, f64, u64)> {
    // Expected format: [download]  42.5% of ~ 150.00MiB at  5.20MiB/s ETA 00:15
    if !line.contains('%') {
        return None;
    }

    // 1. Parse Percentage
    let pct_pos = line.find('%')?;
    let before = &line[..pct_pos];
    let mut num_start = before.len();
    for (i, c) in before.chars().rev().enumerate() {
        if c.is_ascii_digit() || c == '.' {
            num_start = before.len() - i - 1;
        } else if num_start != before.len() {
            break;
        }
    }
    if num_start >= before.len() {
        return None;
    }
    let num_str = &before[num_start..].trim();
    let pct = num_str.parse::<f64>().ok()?;

    // 2. Parse Total Size (look for "of " or "of ~ ")
    let mut total_bytes = 0;
    if let Some(of_idx) = line.find(" of ") {
        let after_of = &line[of_idx + 4..];
        // Check for "~" (approximate)
        let size_str_start = if after_of.trim_start().starts_with('~') {
            if let Some(tilde_pos) = after_of.find('~') {
                &after_of[tilde_pos + 1..]
            } else {
                after_of
            }
        } else {
            after_of
        };

        // Find the end of the size string (usually before " at ")
        let size_end = size_str_start.find(" at ").unwrap_or(size_str_start.len());
        let size_token = size_str_start[..size_end].trim();

        // Parse number and unit
        let mut idx = 0;
        for (i, ch) in size_token.chars().enumerate() {
            if ch.is_ascii_digit() || ch == '.' {
                idx = i + 1;
            } else {
                break;
            }
        }

        if idx > 0 {
            if let Ok(num) = size_token[..idx].parse::<f64>() {
                let unit = size_token[idx..].trim();
                total_bytes = match unit {
                    "KiB" => (num * 1024.0) as u64,
                    "MiB" => (num * 1024.0 * 1024.0) as u64,
                    "GiB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
                    "B" | "" => num as u64,
                    _ => num as u64,
                };
            }
        }
    }

    // 3. Parse Speed (look for ' at ' and '/s')
    let mut speed_bps = 0.0;
    if let Some(at_idx) = line.find(" at ") {
        let after = &line[at_idx + 4..];
        if let Some(slash_idx) = after.find("/s") {
            let token = &after[..slash_idx].trim();
            let mut idx = 0;
            for (i, ch) in token.chars().enumerate() {
                if ch.is_ascii_digit() || ch == '.' {
                    idx = i + 1;
                } else {
                    break;
                }
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

    Some((pct, speed_bps, total_bytes))
}

/// Download configuration
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub segments: usize,                // Number of parallel segments (default: 16)
    pub connections_per_segment: usize, // Connections per segment (default: 1)
    pub chunk_size: usize,              // Chunk size for streaming (default: 8192)
    pub retry_attempts: usize,          // Retry attempts per segment (default: 3)
    pub retry_delay: Duration,          // Delay between retries
    pub enable_resume: bool,            // Enable resume capability
    pub request_delay: Duration,        // Delay between segment requests
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

        // CRITICAL FIX: If URL is a YouTube page, bypass HTTP probing entirely.
        // YouTube URLs redirect to HLS manifests when probed via HTTP HEAD/GET,
        // causing the engine to incorrectly report 50-byte manifest as file size.
        if url.contains("youtube.com/watch") || url.contains("youtu.be/") {
            eprintln!("🔀 [ENGINE] YouTube URL detected - bypassing probe, using yt-dlp directly");
            eprintln!("   - Reason: YouTube URLs redirect to HLS manifests during HTTP probing");
            eprintln!("   - Solution: yt-dlp handles YouTube streams natively");
            return self.download_via_ytdlp(url, output_path, progress_tx).await;
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
        eprintln!(
            "✅ [ENGINE] supports_ranges() await returned: {:?}",
            &supports_ranges_res
        );
        let file_size_res = self.get_file_size(url).await;
        eprintln!(
            "✅ [ENGINE] get_file_size() await returned: {:?}",
            &file_size_res
        );

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
        eprintln!(
            "📊 [ENGINE] Initializing progress with file_size={} and segments={}",
            file_size, self.config.segments
        );
        let mut progress = DownloadProgress::new(file_size, self.config.segments);

        // Send initial progress
        eprintln!("📤 [ENGINE] Sending initial progress (based on probed file size)...");
        if let Err(e) = progress_tx.send(progress.clone()).await {
            eprintln!(
                "⚠️ [ENGINE] Failed to send initial progress (probed): {}",
                e
            );
            warn!("Failed to send initial progress: {}", e);
        }

        // Determine download strategy
        if !supports_ranges || file_size < 1024 * 1024 {
            // < 1MB or no range support
            eprintln!("🔀 [ENGINE] Taking path: simple download (no ranges or small file). supports_ranges={}, file_size={}", supports_ranges, file_size);
            eprintln!("📥 [ENGINE] Using simple download (no ranges or small file). supports_ranges={}, file_size={}", supports_ranges, file_size);
            return self.download_simple(url, output_path, progress_tx).await;
        }

        eprintln!(
            "📦 [ENGINE] Using segmented download path (ranges supported and file large enough)"
        );

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
                    )
                    .await;

                    eprintln!(
                        "✅ [ENGINE] download_segment completed for segment {}: success={}",
                        i,
                        result.is_ok()
                    );

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
                    progress.update(total_downloaded, speed);
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
        progress.update_segment(completed_segments);
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
            merge_segments(
                &segments_paths_clone,
                &output_path_clone,
                Some(merge_progress_tx),
            )
            .await
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
        debug!("download_via_ytdlp called for URL: {}", url);

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

        // Prepare command: yt-dlp with explicit progress flags
        eprintln!("🔧 [YT-DLP] Spawning yt-dlp process...");
        let out = output_path.to_string_lossy().to_string();
        let mut cmd = AsyncCommand::new("yt-dlp");
        cmd.arg("-f")
            .arg("best")
            .arg("--newline") // Force newline after each progress line (critical for non-TTY)
            .arg("--no-warnings") // Reduce stderr noise
            .arg("--progress") // Explicitly enable progress output
            .arg("-o")
            .arg(out)
            .arg(url);
        // Combine stderr and stdout to capture all output
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::piped());

        eprintln!("🚀 [YT-DLP] About to spawn yt-dlp command...");
        let mut child = cmd.spawn()?;
        eprintln!("✅ [YT-DLP] Command spawned successfully, checking stderr pipe...");

        // Read stderr for progress lines — spawn a task that holds a clone of the sender
        // Use a handle to track the reader task so we can detect if it completes
        let reader_handle = if let Some(stderr) = child.stderr.take() {
            eprintln!("✅ [YT-DLP] Stderr pipe available, spawning reader task...");
            let progress_for_reader = progress_tx_for_spawn.clone();

            eprintln!("🎬 [YT-DLP] About to call tokio::spawn for stderr reader...");
            let handle = tokio::spawn(async move {
                eprintln!("📖 [YT-DLP] INSIDE SPAWNED TASK - stderr reader starting!");
                use tokio::io::AsyncBufReadExt;
                let reader = tokio::io::BufReader::new(stderr);
                let mut lines = reader.lines();

                eprintln!("📖 [YT-DLP] Starting stderr reader...");
                let mut line_count = 0;

                while let Ok(Some(line)) = lines.next_line().await {
                    line_count += 1;
                    eprintln!("📄 [YT-DLP] stderr #{}: {}", line_count, line);

                    // Detect errors from yt-dlp
                    if line.contains("ERROR:") || line.contains("error:") {
                        eprintln!("❌ [YT-DLP] Error detected: {}", line);
                        let mut p = DownloadProgress::new(100, 1);
                        p.status = DownloadStatus::Failed(line.clone());
                        let _ = progress_for_reader.send(p).await;
                        break;
                    }

                    // Try to parse percentage and speed from lines like:
                    // [download]  12.5% of ~10.50MiB at 1.23MiB/s ETA 00:07
                    if let Some((pct, speed_bps, total_bytes)) = parse_yt_dlp_progress(&line) {
                        eprintln!(
                            "🔁 [YT-DLP] Parsed progress: {}% speed={} B/s total={} B",
                            pct, speed_bps, total_bytes
                        );
                        let mut p = DownloadProgress::new(100, 1);
                        p.status = DownloadStatus::Downloading;
                        // Calculate downloaded bytes from percentage and total
                        p.downloaded_bytes = if total_bytes > 0 {
                            (pct / 100.0 * total_bytes as f64) as u64
                        } else {
                            pct.round() as u64
                        };
                        p.total_bytes = total_bytes;
                        p.speed = speed_bps;
                        p.segments_completed = 0;

                        // Best-effort send: don't treat a closed receiver as fatal here
                        if let Err(e) = progress_for_reader.send(p).await {
                            eprintln!("⚠️ [YT-DLP] Failed to send parsed progress: {}", e);
                            break;
                        }
                    }
                }

                eprintln!(
                    "📖 [YT-DLP] Stderr reader finished. Read {} lines total",
                    line_count
                );
            });
            eprintln!("✅ [YT-DLP] tokio::spawn returned, reader task is now running");
            Some(handle)
        } else {
            eprintln!(
                "❌ [YT-DLP] ERROR: No stderr pipe available - child.stderr.take() returned None!"
            );
            eprintln!("   This means stderr was not properly set up or was already taken");
            None
        };

        // Also consume stdout and parse progress
        if let Some(stdout) = child.stdout.take() {
            let progress_for_stdout = progress_tx_for_spawn.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let reader = tokio::io::BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    // Try to parse percentage and speed from stdout
                    if let Some((pct, speed_bps, total_bytes)) = parse_yt_dlp_progress(&line) {
                        let mut p = DownloadProgress::new(100, 1);
                        p.status = DownloadStatus::Downloading;
                        p.downloaded_bytes = if total_bytes > 0 {
                            (pct / 100.0 * total_bytes as f64) as u64
                        } else {
                            pct.round() as u64
                        };
                        p.total_bytes = total_bytes;
                        p.speed = speed_bps;
                        p.segments_completed = 0;

                        let _ = progress_for_stdout.send(p).await;
                    }
                }
            });
        }

        // Wait for child to finish with 30-minute timeout
        // Yield to allow spawned tasks to start
        tokio::task::yield_now().await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let wait_result = timeout(TokioDuration::from_secs(1800), child.wait()).await;

        let status = match wait_result {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to wait for yt-dlp process: {}", e)),
            Err(_) => {
                let _ = child.kill().await;
                return Err(anyhow::anyhow!("yt-dlp process timed out after 30 minutes"));
            }
        };

        // Give the reader task a moment to finish reading any buffered output
        if let Some(handle) = reader_handle {
            eprintln!("⏳ [YT-DLP] Waiting for stderr reader to finish...");
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
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
                let speed = if elapsed > 0.0 {
                    downloaded as f64 / elapsed
                } else {
                    0.0
                };

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
        let speed = if elapsed > 0.0 {
            downloaded as f64 / elapsed
        } else {
            0.0
        };

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
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client.head(url).send(),
        )
        .await
        {
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
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client.head(url).send(),
        )
        .await
        {
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
