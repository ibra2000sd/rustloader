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
use crate::downloader::progress::{
    DownloadProgress, DownloadStatus, StallDetector, STALL_DETECTION_SECONDS,
};
use crate::downloader::resume_guard::{
    read_sidecar, remove_sidecar, sidecar_path, write_sidecar, ResumeIdentity,
};
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
use tracing::{debug, error, info, warn};

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

/// Options that tune the yt-dlp invocation used for YouTube / HLS / complex
/// sources.
///
/// Semantics ported from the legacy `rustloader2` CLI (`src/cli.rs`,
/// `src/downloader.rs`). These map the CLI's ergonomic flags onto the yt-dlp
/// arguments the engine already shells out with — they do **not** introduce a
/// second download code path. `YtDlpOptions::default()` reproduces the engine's
/// historical invocation exactly.
#[derive(Debug, Clone, Default)]
pub struct YtDlpOptions {
    /// Maximum video height (e.g. 480/720/1080). `None` => best available.
    pub quality: Option<u32>,
    /// Extract audio only (maps to yt-dlp `-x`).
    pub audio_only: bool,
    /// Audio format when extracting audio (e.g. "mp3").
    pub audio_format: Option<String>,
    /// Download subtitles (`--write-subs --sub-langs all`).
    pub subtitles: bool,
    /// Download the whole playlist (`--yes-playlist`).
    pub playlist: bool,
    /// Clip start time (e.g. 00:01:00).
    pub start_time: Option<String>,
    /// Clip end time (e.g. 00:02:00).
    pub end_time: Option<String>,
    /// Audio bitrate passed to the ffmpeg postprocessor (e.g. "128K").
    pub audio_bitrate: Option<String>,
    /// Cookie source for sites that require authentication (e.g. YouTube's
    /// anti-bot check). Default (empty) emits no cookie arguments.
    pub cookies: crate::utils::CookieConfig,
}

/// Build the yt-dlp argument vector for the given options, URL and output path.
///
/// With [`YtDlpOptions::default()`] this returns exactly the engine's historical
/// argument list (`-f best --newline --no-warnings --progress -o <out> <url>`),
/// so the GUI download path is unchanged. Extra arguments are only emitted when
/// the corresponding option is set, which is how the CLI flags become real
/// behaviour instead of cosmetics.
pub fn build_ytdlp_args(opts: &YtDlpOptions, url: &str, output: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    if opts.audio_only {
        args.push("-x".to_string());
        if let Some(fmt) = &opts.audio_format {
            args.push("--audio-format".to_string());
            args.push(fmt.clone());
        }
    } else {
        let selector = match opts.quality {
            Some(h) => {
                format!("bestvideo[height<={h}]+bestaudio/best[height<={h}]/best")
            }
            // A bare `best` makes yt-dlp reject HLS master playlists with
            // "Requested format is not available". `bestvideo*+bestaudio/best`
            // resolves HLS/DASH variants (merging video+audio when split) and
            // still falls back to a single progressive stream via `/best`.
            None => "bestvideo*+bestaudio/best".to_string(),
        };
        args.push("-f".to_string());
        args.push(selector);
    }

    if let Some(bitrate) = &opts.audio_bitrate {
        args.push("--postprocessor-args".to_string());
        args.push(format!("ffmpeg:-b:a {bitrate}"));
    }

    if opts.subtitles {
        args.push("--write-subs".to_string());
        args.push("--sub-langs".to_string());
        args.push("all".to_string());
    }

    if opts.playlist {
        args.push("--yes-playlist".to_string());
    }

    if opts.start_time.is_some() || opts.end_time.is_some() {
        let start = opts.start_time.clone().unwrap_or_else(|| "0".to_string());
        let end = opts.end_time.clone().unwrap_or_else(|| "inf".to_string());
        args.push("--download-sections".to_string());
        args.push(format!("*{start}-{end}"));
    }

    // Cookie source (if configured) for authenticated sites. Default config
    // appends nothing, so the historical argument list is unchanged.
    opts.cookies.append_args(&mut args);

    args.push("--newline".to_string());
    args.push("--no-warnings".to_string());
    args.push("--progress".to_string());
    args.push("-o".to_string());
    args.push(output.to_string());
    args.push(url.to_string());

    args
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
    ytdlp_options: YtDlpOptions,
}

impl DownloadEngine {
    /// Create new download engine with configuration
    pub fn new(config: DownloadConfig) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            ytdlp_options: YtDlpOptions::default(),
        }
    }

    /// Configure the yt-dlp options used by the yt-dlp download path
    /// (builder-style). Defaults preserve the engine's historical behaviour.
    pub fn with_ytdlp_options(mut self, options: YtDlpOptions) -> Self {
        self.ytdlp_options = options;
        self
    }

    /// Download file with progress tracking
    pub async fn download(
        &self,
        url: &str,
        output_path: &Path,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        debug!("🚀🚀🚀 [ENGINE-ENTRY] download() ENTERED - First line executed!");
        debug!("    URL: {}", url);
        debug!("    Output: {:?}", output_path);

        // Send a conservative initial progress so GUI can show a task entry immediately
        debug!("📤 [ENGINE] Sending initial progress (Initializing)...");
        let mut initial = DownloadProgress::new(0, 1);
        initial.status = DownloadStatus::Initializing;
        if let Err(e) = progress_tx.send(initial.clone()).await {
            error!("⚠️ [ENGINE] Failed to send initial progress: {}", e);
            warn!("Failed to send initial progress: {}", e);
        } else {
            debug!("✅ [ENGINE] Initial progress sent");
        }

        // Route on WHAT THE URL ACTUALLY IS, not its site name. Probe the URL
        // with a SINGLE ranged GET (`Range: bytes=0-0`) and let the response's
        // Content-Type decide the path:
        //   • a direct media stream (video/*, audio/*, octet-stream) → native
        //     engine (segmented or simple);
        //   • anything else — HTML pages, HLS/DASH manifests, unknown types — or
        //     a probe failure → yt-dlp, which resolves and muxes the real streams.
        // yt-dlp runs on the page URL itself, so every yt-dlp-supported site (and
        // HLS/DASH) is handled here with NO per-site special-casing, and a 200
        // HTML body is never mistaken for a media file.
        //
        // The ranged GET is also HEAD-independent: HEAD-based probing was
        // unreliable (reqwest's `content_length()` on a HEAD response reflects the
        // empty body, not the `Content-Length` header, so size came back 0 and the
        // segmented path was never taken).
        debug!(
            "🔍 [ENGINE] Probing server (ranged GET) for range support, size, and content type..."
        );
        let probe = match self.probe(url).await {
            Ok(p) => {
                debug!(
                    "   - supports_ranges={}, file_size={}, content_type={:?}",
                    p.supports_ranges, p.size, p.content_type
                );
                p
            }
            Err(e) => {
                info!("🔀 [ENGINE] Taking path: yt-dlp fallback (probe failed)");
                warn!("⚠️ [ENGINE] Probe failed, falling back to yt-dlp: {}", e);
                return self.download_via_ytdlp(url, output_path, progress_tx).await;
            }
        };

        // Content-Type-based routing: anything that isn't a direct media stream
        // (HTML pages, HLS/DASH manifests, unknown types) goes to yt-dlp.
        if !is_direct_media(probe.content_type.as_deref()) {
            info!(
                "🔀 [ENGINE] Taking path: yt-dlp (not a direct media URL; content_type={:?})",
                probe.content_type
            );
            return self.download_via_ytdlp(url, output_path, progress_tx).await;
        }

        info!(
            "🔀 [ENGINE] Direct media URL (content_type={:?}) - using native engine",
            probe.content_type
        );
        let (supports_ranges, file_size) = (probe.supports_ranges, probe.size);

        // Initialize progress
        info!(
            "📊 [ENGINE] Initializing progress with file_size={} and segments={}",
            file_size, self.config.segments
        );
        let mut progress = DownloadProgress::new(file_size, self.config.segments);

        // Send initial progress
        debug!("📤 [ENGINE] Sending initial progress (based on probed file size)...");
        if let Err(e) = progress_tx.send(progress.clone()).await {
            warn!(
                "⚠️ [ENGINE] Failed to send initial progress (probed): {}",
                e
            );
            warn!("Failed to send initial progress: {}", e);
        }

        // Determine download strategy
        if !supports_ranges || file_size < 1024 * 1024 {
            // < 1MB or no range support
            info!("🔀 [ENGINE] Taking path: simple download (no ranges or small file). supports_ranges={}, file_size={}", supports_ranges, file_size);
            info!("📥 [ENGINE] Using simple download (no ranges or small file). supports_ranges={}, file_size={}", supports_ranges, file_size);
            return self.download_simple(url, output_path, progress_tx).await;
        }

        info!("📦 [ENGINE] Using segmented download path (ranges supported and file large enough)");

        // Calculate segments
        let segments = calculate_segments(file_size, self.config.segments, output_path);
        progress.total_segments = segments.len();

        // Cross-session resume identity guard (F-DL-003): #28/#29 made a
        // segment's resume-from-written-bytes safe against a range that's
        // silently ignored by the server, but say nothing about whether the
        // `.partN` files on disk actually belong to *this* download's plan —
        // a segment-count change between sessions, or a different download
        // reusing this same `output_path`, would otherwise get silently
        // appended into (wrong offsets, or a foreign file's bytes spliced
        // in). Require a sidecar identity match (URL + file_size +
        // segment_count) before trusting any existing part; on any mismatch,
        // or when resume is disabled, discard this plan's parts so the
        // segment loop below starts clean instead of corrupting silently.
        let resume_sidecar = sidecar_path(output_path);
        if self.config.enable_resume {
            let current_identity = ResumeIdentity::new(url, file_size, self.config.segments);
            let trusted = read_sidecar(&resume_sidecar).await.as_ref() == Some(&current_identity);
            if !trusted {
                let stale_paths: Vec<PathBuf> = segments.iter().map(|s| s.path.clone()).collect();
                if let Err(e) = cleanup_segments(&stale_paths).await {
                    warn!("Failed to discard stale/foreign segment parts: {}", e);
                }
            }
            if let Err(e) = write_sidecar(&resume_sidecar, &current_identity).await {
                warn!("Failed to write resume identity sidecar: {}", e);
            }
        } else {
            let stale_paths: Vec<PathBuf> = segments.iter().map(|s| s.path.clone()).collect();
            if let Err(e) = cleanup_segments(&stale_paths).await {
                warn!("Failed to discard segment parts (resume disabled): {}", e);
            }
            remove_sidecar(&resume_sidecar).await;
        }

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
                        debug!("✅ [ENGINE] Sleep before starting segment {} completed", i);
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

                    debug!(
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
                        warn!("⚠️ [ENGINE] Failed to send aggregated progress: {}", e);
                        warn!("Failed to send progress update: {}", e);
                        break;
                    } else {
                        debug!("✅ [ENGINE] Sent aggregated progress to progress_tx_clone");
                    }

                    last_update_time = now;
                    last_downloaded = total_downloaded;
                }
            }
        });

        // Stall watchdog: classify the download as stalled and surface a
        // `DownloadStatus::Stalled` event if no forward progress is observed
        // within the stall threshold (`STALL_DETECTION_SECONDS`). This is the
        // automatic stall detection the engine previously lacked; it flows
        // through the same `progress_tx` the GUI and CLI already consume.
        let stall_progress = Arc::clone(&segment_progress_clone);
        let stall_tx = progress_tx.clone();
        let stall_watchdog = tokio::spawn(async move {
            let mut detector = StallDetector::new();
            let mut ticker = tokio::time::interval(Duration::from_secs(5));
            ticker.tick().await; // consume the immediate first tick
            let mut reported = false;
            loop {
                ticker.tick().await;
                let downloaded: u64 = { stall_progress.lock().await.iter().sum() };
                if detector.record(downloaded) {
                    // Progress resumed; allow a fresh stall report later.
                    reported = false;
                } else if detector.is_stalled() && !reported {
                    reported = true;
                    warn!(
                        "⏱️ [ENGINE] Download stalled (no progress for {STALL_DETECTION_SECONDS}s)"
                    );
                    let mut progress = DownloadProgress::new(file_size, segments_count);
                    progress.downloaded_bytes = downloaded;
                    progress.stalled();
                    if stall_tx.send(progress).await.is_err() {
                        break;
                    }
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

        // Abort the progress + stall-watchdog tasks
        segment_progress_task.abort();
        stall_watchdog.abort();

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
        remove_sidecar(&resume_sidecar).await;

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

        debug!("📤 [YT-DLP] Sending initial 0% progress (best-effort)");
        let mut initial = DownloadProgress::new(0, 1);
        initial.status = DownloadStatus::Downloading;
        initial.downloaded_bytes = 0;
        initial.speed = 0.0;
        // Best-effort: don't propagate an error if receiver was dropped.
        let _ = progress_tx_for_spawn.send(initial.clone()).await;

        // Prepare command: yt-dlp with explicit progress flags. Arguments are
        // built from the configured options; with default options this is the
        // historical `-f best --newline --no-warnings --progress -o <out> <url>`.
        debug!("🔧 [YT-DLP] Spawning yt-dlp process...");
        let out = output_path.to_string_lossy().to_string();
        let args = build_ytdlp_args(&self.ytdlp_options, url, &out);
        debug!("🔧 [YT-DLP] Args: {:?}", args);
        let mut cmd = AsyncCommand::new("yt-dlp");
        cmd.args(&args);
        // Combine stderr and stdout to capture all output
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::piped());

        debug!("🚀 [YT-DLP] About to spawn yt-dlp command...");
        let mut child = cmd.spawn()?;
        debug!("✅ [YT-DLP] Command spawned successfully, checking stderr pipe...");

        // Read stderr for progress lines — spawn a task that holds a clone of the sender
        // Use a handle to track the reader task so we can detect if it completes
        let reader_handle = if let Some(stderr) = child.stderr.take() {
            debug!("✅ [YT-DLP] Stderr pipe available, spawning reader task...");
            let progress_for_reader = progress_tx_for_spawn.clone();

            debug!("🎬 [YT-DLP] About to call tokio::spawn for stderr reader...");
            let handle = tokio::spawn(async move {
                debug!("📖 [YT-DLP] INSIDE SPAWNED TASK - stderr reader starting!");
                use tokio::io::AsyncBufReadExt;
                let reader = tokio::io::BufReader::new(stderr);
                let mut lines = reader.lines();

                debug!("📖 [YT-DLP] Starting stderr reader...");
                let mut line_count = 0;

                while let Ok(Some(line)) = lines.next_line().await {
                    line_count += 1;
                    debug!("📄 [YT-DLP] stderr #{}: {}", line_count, line);

                    // Detect errors from yt-dlp
                    if line.contains("ERROR:") || line.contains("error:") {
                        error!("❌ [YT-DLP] Error detected: {}", line);
                        let mut p = DownloadProgress::new(100, 1);
                        p.status = DownloadStatus::Failed(line.clone());
                        let _ = progress_for_reader.send(p).await;
                        break;
                    }

                    // Try to parse percentage and speed from lines like:
                    // [download]  12.5% of ~10.50MiB at 1.23MiB/s ETA 00:07
                    if let Some((pct, speed_bps, total_bytes)) = parse_yt_dlp_progress(&line) {
                        debug!(
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
                            warn!("⚠️ [YT-DLP] Failed to send parsed progress: {}", e);
                            break;
                        }
                    }
                }

                debug!(
                    "📖 [YT-DLP] Stderr reader finished. Read {} lines total",
                    line_count
                );
            });
            debug!("✅ [YT-DLP] tokio::spawn returned, reader task is now running");
            Some(handle)
        } else {
            error!(
                "❌ [YT-DLP] ERROR: No stderr pipe available - child.stderr.take() returned None!"
            );
            error!("   This means stderr was not properly set up or was already taken");
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
            debug!("⏳ [YT-DLP] Waiting for stderr reader to finish...");
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        debug!("🔚 [YT-DLP] Process exited with: {:?}", status.code());
        if status.success() {
            info!("✅ [YT-DLP] Download successful");
            let mut done = DownloadProgress::new(0, 1);
            done.status = DownloadStatus::Completed;
            done.downloaded_bytes = 0;
            done.speed = 0.0;
            done.complete();
            let _ = progress_tx.send(done).await;
            Ok(())
        } else {
            error!("❌ [YT-DLP] Download failed");
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

        // Defensive Content-Type guard: never write a non-media response (e.g. an
        // HTML error/interstitial page returned with 200) as the output media
        // file. Routing already filters non-media URLs to yt-dlp, but this is the
        // last line of defence against silent corruption.
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        if !is_direct_media(content_type.as_deref()) {
            return Err(anyhow::anyhow!(
                "refusing to write non-media response as a media file (content_type={:?})",
                content_type
            ));
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

    /// Probe a URL for range support and total size in a single request.
    ///
    /// Sends a ranged `GET` (`Range: bytes=0-0`) and interprets the response:
    /// - `206 Partial Content` → ranges supported; the total size is the value
    ///   after `/` in the `Content-Range` header (`bytes 0-0/<total>`).
    /// - `200 OK` (server ignored `Range`) → ranges not supported; the size is
    ///   read from the `Content-Length` **header** (not `Response::content_length`,
    ///   which would be the body length).
    ///
    /// Returns a [`ProbeResult`] carrying range support, total size, and the
    /// response `Content-Type`. `size` is `0` when the server didn't advertise a
    /// usable length, which makes the caller fall back to the simple
    /// (non-segmented) download path; `content_type` drives media-vs-yt-dlp
    /// routing (see [`is_direct_media`]).
    async fn probe(&self, url: &str) -> Result<ProbeResult> {
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client.get(url).header("Range", "bytes=0-0").send(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("probe request timeout (10s)"))?
        .map_err(|e| anyhow::anyhow!("probe request failed: {}", e))?;

        let status = response.status();
        let headers = response.headers();

        // Capture the Content-Type for routing (independent of range support).
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if status == reqwest::StatusCode::PARTIAL_CONTENT {
            // 206: ranges supported. Parse the total from `bytes 0-0/<total>`.
            let total = headers
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit('/').next())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            debug!(
                "Probe: ranges supported (206), total_size={} (from Content-Range), content_type={:?}",
                total, content_type
            );
            Ok(ProbeResult {
                supports_ranges: true,
                size: total,
                content_type,
            })
        } else if status.is_success() {
            // 200: server ignored Range. Read the header directly.
            let size = headers
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            debug!(
                "Probe: server ignored Range (200), size={} (from Content-Length header), content_type={:?}",
                size, content_type
            );
            Ok(ProbeResult {
                supports_ranges: false,
                size,
                content_type,
            })
        } else {
            Err(anyhow::anyhow!("probe got unexpected status {}", status))
        }
    }
}

/// Outcome of probing a URL with a single ranged GET: whether the server
/// supports byte ranges, the total size (`0` if unknown), and the response
/// `Content-Type` (used by [`is_direct_media`] to route media vs yt-dlp).
#[derive(Debug, Clone)]
struct ProbeResult {
    supports_ranges: bool,
    size: u64,
    content_type: Option<String>,
}

/// Decide whether a `Content-Type` denotes a directly-downloadable media stream
/// the native engine can fetch.
///
/// Returns `true` only for `video/*`, `audio/*`, and `application/octet-stream`
/// (a generic binary commonly used for direct media files). Everything else —
/// HTML pages, HLS/DASH manifests (`application/x-mpegURL`,
/// `application/vnd.apple.mpegurl`, `application/dash+xml`), any other type, or a
/// missing header — is treated as "not a direct media file" and routed to yt-dlp,
/// which resolves the real streams. This is what makes engine coverage equal
/// yt-dlp's without any per-site logic.
fn is_direct_media(content_type: Option<&str>) -> bool {
    match content_type {
        Some(ct) => {
            // Strip any `; charset=...`/parameters and normalise case.
            let main = ct
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            // HLS/DASH manifests are NOT direct media and must go to yt-dlp — even
            // when served with an `audio/...` mpegurl type (e.g. `audio/mpegurl`,
            // `audio/x-mpegurl`) that would otherwise match the `audio/` prefix
            // below. Catch every mpegurl spelling and DASH manifests explicitly.
            if main.contains("mpegurl") || main.contains("dash+xml") {
                return false;
            }
            main.starts_with("video/")
                || main.starts_with("audio/")
                || main == "application/octet-stream"
        }
        None => false,
    }
}

impl Default for DownloadEngine {
    fn default() -> Self {
        Self::new(DownloadConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // CONFIGURATION TESTS
    // ============================================================

    #[test]
    fn test_download_config_defaults() {
        let config = DownloadConfig::default();
        assert_eq!(config.segments, 16, "Default should be 16 segments");
        assert_eq!(
            config.connections_per_segment, 1,
            "Default connections per segment should be 1"
        );
        assert_eq!(config.chunk_size, 8192, "Default chunk size should be 8KB");
        assert_eq!(
            config.retry_attempts, 3,
            "Default retry attempts should be 3"
        );
        assert!(config.enable_resume, "Resume should be enabled by default");
    }

    #[test]
    fn test_download_config_custom() {
        let config = DownloadConfig {
            segments: 8,
            connections_per_segment: 2,
            chunk_size: 16384,
            retry_attempts: 5,
            retry_delay: Duration::from_secs(5),
            enable_resume: false,
            request_delay: Duration::from_millis(200),
        };

        assert_eq!(config.segments, 8);
        assert_eq!(config.connections_per_segment, 2);
        assert_eq!(config.chunk_size, 16384);
        assert_eq!(config.retry_attempts, 5);
        assert!(!config.enable_resume);
    }

    // ============================================================
    // ENGINE INITIALIZATION TESTS
    // ============================================================

    #[test]
    fn test_download_engine_creation() {
        let config = DownloadConfig::default();
        let engine = DownloadEngine::new(config);
        assert_eq!(engine.config.segments, 16);
    }

    #[test]
    fn test_download_engine_default() {
        let engine = DownloadEngine::default();
        assert_eq!(engine.config.segments, 16);
        assert!(engine.config.enable_resume);
    }

    #[test]
    fn test_download_engine_with_custom_config() {
        let config = DownloadConfig {
            segments: 4,
            retry_attempts: 10,
            ..Default::default()
        };
        let engine = DownloadEngine::new(config);
        assert_eq!(engine.config.segments, 4);
        assert_eq!(engine.config.retry_attempts, 10);
    }

    // ============================================================
    // YT-DLP PROGRESS PARSING TESTS
    // ============================================================

    #[test]
    fn test_parse_yt_dlp_progress_typical() {
        let line = "[download]  42.5% of ~ 150.00MiB at  5.20MiB/s ETA 00:15";
        let result = parse_yt_dlp_progress(line);
        assert!(result.is_some());

        let (pct, speed, total) = result.unwrap();
        assert!((pct - 42.5).abs() < 0.1, "Percentage should be ~42.5");
        assert!(speed > 5_000_000.0, "Speed should be ~5.2 MiB/s");
        assert!(total > 150_000_000, "Total should be ~150 MiB");
    }

    #[test]
    fn test_parse_yt_dlp_progress_no_percentage() {
        let line = "[download] Downloading video...";
        let result = parse_yt_dlp_progress(line);
        assert!(result.is_none(), "Should return None for lines without %");
    }

    #[test]
    fn test_parse_yt_dlp_progress_with_kilobytes() {
        let line = "[download]  10.0% of 500.00KiB at  50.00KiB/s";
        let result = parse_yt_dlp_progress(line);
        assert!(result.is_some());

        let (pct, speed, total) = result.unwrap();
        assert!((pct - 10.0).abs() < 0.1);
        assert!(speed > 50_000.0, "Speed should be ~50 KiB/s");
        assert!(total > 500_000, "Total should be ~500 KiB");
    }

    #[test]
    fn test_parse_yt_dlp_progress_with_gigabytes() {
        let line = "[download]  75.0% of 2.00GiB at  10.00MiB/s";
        let result = parse_yt_dlp_progress(line);
        assert!(result.is_some());

        let (pct, speed, total) = result.unwrap();
        assert!((pct - 75.0).abs() < 0.1);
        assert!(speed > 10_000_000.0, "Speed should be ~10 MiB/s");
        assert!(total > 2_000_000_000, "Total should be ~2 GiB");
    }

    #[test]
    fn test_parse_yt_dlp_progress_approximate_size() {
        let line = "[download]  50.0% of ~ 100.00MiB at  2.00MiB/s";
        let result = parse_yt_dlp_progress(line);
        assert!(result.is_some(), "Should handle approximate sizes with ~");
    }

    #[test]
    fn test_parse_yt_dlp_progress_malformed() {
        let line = "[download] Something % weird";
        let result = parse_yt_dlp_progress(line);
        // May return None or Some with zeroed values - either is acceptable
        // The important thing is it doesn't panic
    }

    // ============================================================
    // DOWNLOAD ENGINE FUNCTIONAL TESTS
    // ============================================================

    // ============================================================
    // CONTENT-TYPE ROUTING CLASSIFIER TESTS
    // ============================================================

    #[test]
    fn test_is_direct_media_accepts_media_types() {
        assert!(is_direct_media(Some("video/mp4")));
        assert!(is_direct_media(Some("video/webm")));
        assert!(is_direct_media(Some("audio/mpeg")));
        assert!(is_direct_media(Some("audio/mp4")));
        assert!(is_direct_media(Some("application/octet-stream")));
        // Parameters and odd casing must not defeat the match.
        assert!(is_direct_media(Some("video/mp4; charset=binary")));
        assert!(is_direct_media(Some("Video/MP4")));
        assert!(is_direct_media(Some("  audio/ogg  ")));
    }

    #[test]
    fn test_is_direct_media_rejects_non_media_types() {
        // HTML pages (the silent-corruption case).
        assert!(!is_direct_media(Some("text/html")));
        assert!(!is_direct_media(Some("text/html; charset=utf-8")));
        // HLS / DASH manifests must go to yt-dlp, not the native engine —
        // including the `audio/...` mpegurl spellings that start with `audio/`.
        assert!(!is_direct_media(Some("application/x-mpegURL")));
        assert!(!is_direct_media(Some("application/vnd.apple.mpegurl")));
        assert!(!is_direct_media(Some("application/dash+xml")));
        assert!(!is_direct_media(Some("audio/mpegurl")));
        assert!(!is_direct_media(Some("audio/x-mpegurl")));
        assert!(!is_direct_media(Some("application/mpegurl")));
        assert!(!is_direct_media(Some("audio/mpegurl; charset=utf-8")));
        // Other / unknown / missing.
        assert!(!is_direct_media(Some("application/json")));
        assert!(!is_direct_media(Some("application/xml")));
        assert!(!is_direct_media(Some("")));
        assert!(!is_direct_media(None));
    }

    #[tokio::test]
    async fn test_engine_supports_ranges_mock() {
        // This test would require a mock server
        // For now, we test that the function exists and has proper signature
        let engine = DownloadEngine::default();
        // In a real test environment, you'd use a mock server
        // For unit testing, we verify the structure exists
        assert_eq!(engine.config.segments, 16);
    }

    #[tokio::test]
    async fn test_engine_get_file_size_mock() {
        // Similar to above - would require mock server
        let engine = DownloadEngine::default();
        assert!(engine.config.retry_attempts > 0);
    }

    // ============================================================
    // INTEGRATION-STYLE TESTS (requires real resources)
    // ============================================================

    // Note: Full download tests are in integration tests
    // These are structure and configuration validation tests

    #[test]
    fn test_download_config_validation() {
        let config = DownloadConfig {
            segments: 32,
            chunk_size: 1024,
            retry_attempts: 0, // Edge case: no retries
            ..Default::default()
        };

        assert_eq!(config.segments, 32);
        assert_eq!(config.retry_attempts, 0, "Should allow 0 retry attempts");
    }

    // ============================================================
    // YT-DLP ARGUMENT BUILDER TESTS
    // ============================================================

    #[test]
    fn test_build_ytdlp_args_default_selector() {
        // The no-quality default must use a robust selector that also resolves
        // HLS/DASH variants — a bare `best` makes yt-dlp reject HLS master
        // playlists ("Requested format is not available").
        let opts = YtDlpOptions::default();
        let args = build_ytdlp_args(&opts, "https://example.com/v", "/tmp/out.mp4");
        assert_eq!(
            args,
            vec![
                "-f",
                "bestvideo*+bestaudio/best",
                "--newline",
                "--no-warnings",
                "--progress",
                "-o",
                "/tmp/out.mp4",
                "https://example.com/v",
            ]
        );
        // `-f` is still present and the selector is the robust chain (not bare best).
        assert!(args.iter().any(|a| a == "-f"));
        assert_eq!(args[1], "bestvideo*+bestaudio/best");
    }

    #[test]
    fn test_build_ytdlp_args_quality_selector() {
        let opts = YtDlpOptions {
            quality: Some(720),
            ..Default::default()
        };
        let args = build_ytdlp_args(&opts, "URL", "/out.mp4");
        assert!(args.iter().any(|a| a == "-f"));
        assert!(
            args.iter().any(|a| a.contains("height<=720")),
            "quality must produce a height-capped selector: {args:?}"
        );
    }

    #[test]
    fn test_build_ytdlp_args_audio_only() {
        let opts = YtDlpOptions {
            audio_only: true,
            audio_format: Some("mp3".to_string()),
            audio_bitrate: Some("128K".to_string()),
            ..Default::default()
        };
        let args = build_ytdlp_args(&opts, "URL", "/out.mp3");
        assert!(args.iter().any(|a| a == "-x"));
        assert!(args.windows(2).any(|w| w == ["--audio-format", "mp3"]));
        assert!(args.iter().any(|a| a.contains("ffmpeg:-b:a 128K")));
        // No video format selector when extracting audio.
        assert!(!args.iter().any(|a| a == "-f"));
    }

    #[test]
    fn test_build_ytdlp_args_subs_and_playlist() {
        let opts = YtDlpOptions {
            subtitles: true,
            playlist: true,
            ..Default::default()
        };
        let args = build_ytdlp_args(&opts, "URL", "/out.mp4");
        assert!(args.iter().any(|a| a == "--write-subs"));
        assert!(args.windows(2).any(|w| w == ["--sub-langs", "all"]));
        assert!(args.iter().any(|a| a == "--yes-playlist"));
    }

    #[test]
    fn test_build_ytdlp_args_sections() {
        let opts = YtDlpOptions {
            start_time: Some("00:00:10".to_string()),
            end_time: Some("00:00:20".to_string()),
            ..Default::default()
        };
        let args = build_ytdlp_args(&opts, "URL", "/out.mp4");
        assert!(args
            .windows(2)
            .any(|w| w == ["--download-sections", "*00:00:10-00:00:20"]));
    }

    #[test]
    fn test_engine_with_ytdlp_options_builder() {
        let opts = YtDlpOptions {
            quality: Some(1080),
            ..Default::default()
        };
        let engine = DownloadEngine::default().with_ytdlp_options(opts);
        assert_eq!(engine.ytdlp_options.quality, Some(1080));
    }

    #[test]
    fn test_download_config_extreme_values() {
        let config = DownloadConfig {
            segments: 1,         // Minimum segments
            chunk_size: 1,       // Minimum chunk
            retry_attempts: 100, // High retry count
            ..Default::default()
        };

        assert_eq!(config.segments, 1);
        assert_eq!(config.chunk_size, 1);
        assert_eq!(config.retry_attempts, 100);
    }

    // ============================================================
    // CROSS-SESSION RESUME IDENTITY GUARD TESTS (F-DL-003, Shape 2)
    // ============================================================
    //
    // These exercise `DownloadEngine::download()` end-to-end against a real
    // hand-rolled `tokio::net::TcpListener` HTTP/1.1 server (same idiom as
    // #28/#29's `segment.rs` mock server: no new dev-dependency, precise
    // control over what's served). Unlike the segment-level tests, this one
    // must correctly honor whatever `Range` it's asked for (not just serve
    // "from start to end of body"), because the engine's one-byte probe
    // (`bytes=0-0`) and real per-segment ranges must both be answered
    // correctly for the segmented path to even be taken.

    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    /// Serves ranged GET requests for `body`, honoring the requested
    /// `Range: bytes=X-Y` (or `bytes=X-`) precisely. Tallies the total bytes
    /// actually served into the returned counter, so a test can prove
    /// whether a run resumed (served less than the full body) or re-fetched
    /// everything from scratch (served exactly the full body plus the
    /// one-byte probe).
    async fn spawn_ranged_media_server(
        body: Vec<u8>,
    ) -> (String, Arc<AtomicU64>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let body = Arc::new(body);
        let served = Arc::new(AtomicU64::new(0));
        let served_for_task = Arc::clone(&served);

        let handle = tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let body = Arc::clone(&body);
                let served = Arc::clone(&served_for_task);

                tokio::spawn(async move {
                    let mut buf = [0u8; 8192];
                    let mut req = Vec::new();
                    loop {
                        let n = match socket.read(&mut buf).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => n,
                        };
                        req.extend_from_slice(&buf[..n]);
                        if req.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }

                    let req_str = String::from_utf8_lossy(&req);
                    let last = body.len().saturating_sub(1);
                    let (start, end) = req_str
                        .lines()
                        .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                        .and_then(|l| l.split('=').nth(1))
                        .map(|spec| {
                            let mut parts = spec.splitn(2, '-');
                            let start = parts
                                .next()
                                .unwrap_or("")
                                .trim()
                                .parse::<usize>()
                                .unwrap_or(0);
                            let end = parts
                                .next()
                                .unwrap_or("")
                                .trim()
                                .parse::<usize>()
                                .unwrap_or(last);
                            (start.min(last), end.min(last))
                        })
                        .unwrap_or((0, last));

                    let slice = &body[start..=end];
                    let headers = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        start,
                        end,
                        body.len(),
                        slice.len()
                    );
                    if socket.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    let _ = socket.write_all(slice).await;
                    let _ = socket.flush().await;
                    served.fetch_add(slice.len() as u64, Ordering::SeqCst);
                });
            }
        });

        (format!("http://{}", addr), served, handle)
    }

    fn write_stub_part(path: &Path, data: &[u8]) {
        std::fs::write(path, data).expect("write stub part file");
    }

    #[tokio::test]
    async fn test_resume_trusts_matching_identity_and_skips_written_bytes() {
        let body: Vec<u8> = (0..(12 * 1024 * 1024) as u32)
            .map(|i| (i % 256) as u8)
            .collect();
        let (base_url, served, _server) = spawn_ranged_media_server(body.clone()).await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let output_path = tmp.path().join("out.mp4");

        // Simulate a prior run that got halfway through every segment
        // before being interrupted (pause or app-close) — left on disk,
        // per #28/#29 + F-DL-003's own finding that nothing cleans these up
        // on interruption.
        let segments = calculate_segments(body.len() as u64, 4, &output_path);
        for seg in &segments {
            let half = (seg.size / 2) as usize;
            let start = seg.start as usize;
            write_stub_part(&seg.path, &body[start..start + half]);
        }
        let identity = ResumeIdentity::new(&base_url, body.len() as u64, 4);
        write_sidecar(&sidecar_path(&output_path), &identity)
            .await
            .expect("write sidecar");

        let engine = DownloadEngine::new(DownloadConfig {
            segments: 4,
            retry_attempts: 2,
            retry_delay: Duration::from_millis(5),
            request_delay: Duration::from_millis(1),
            ..Default::default()
        });

        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = engine.download(&base_url, &output_path, tx).await;
        assert!(result.is_ok(), "expected resume to succeed: {:?}", result);

        let output = tokio::fs::read(&output_path).await.expect("read output");
        assert_eq!(output, body, "resumed output must be byte-correct");

        let served_bytes = served.load(Ordering::SeqCst);
        assert!(
            served_bytes < body.len() as u64,
            "expected a genuine resume (less than the full body re-fetched): served {} of {} bytes",
            served_bytes,
            body.len()
        );

        assert!(
            !sidecar_path(&output_path).exists(),
            "sidecar should be removed on successful completion"
        );
    }

    #[tokio::test]
    async fn test_resume_restarts_clean_when_segment_count_changed() {
        // Large enough to land in calculate_segments' 50MB-500MB bracket,
        // where the requested segment count (up to 16) is actually honored —
        // below 50MB it's clamped to at most 4 regardless of config, which
        // wouldn't let this test exercise an 8-segment vs. 4-segment plan.
        let body: Vec<u8> = (0..(55 * 1024 * 1024) as u32)
            .map(|i| (i % 256) as u8)
            .collect();
        let (base_url, served, _server) = spawn_ranged_media_server(body.clone()).await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let output_path = tmp.path().join("out.mp4");

        // Simulate a PRIOR session that left a completed 8-segment plan's
        // parts on disk (same url/file_size — only segment_count differs)
        // plus a sidecar recorded for that 8-segment plan.
        let old_segments = calculate_segments(body.len() as u64, 8, &output_path);
        for seg in &old_segments {
            let start = seg.start as usize;
            let end = seg.end as usize;
            write_stub_part(&seg.path, &body[start..=end]);
        }
        let old_identity = ResumeIdentity::new(&base_url, body.len() as u64, 8);
        write_sidecar(&sidecar_path(&output_path), &old_identity)
            .await
            .expect("write sidecar");

        // THIS session's config uses 4 segments instead — a segment-count
        // preference change between sessions. Segment 0 always starts at
        // byte 0 in both plans (coincidentally safe), but segment 1 onward
        // has different start/end offsets between the two plans, so
        // trusting the old parts here would silently misalign bytes.
        let engine = DownloadEngine::new(DownloadConfig {
            segments: 4,
            retry_attempts: 2,
            retry_delay: Duration::from_millis(5),
            request_delay: Duration::from_millis(1),
            ..Default::default()
        });

        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = engine.download(&base_url, &output_path, tx).await;
        assert!(
            result.is_ok(),
            "expected a clean restart to succeed: {:?}",
            result
        );

        let output = tokio::fs::read(&output_path).await.expect("read output");
        assert_eq!(
            output, body,
            "output must be byte-correct despite mismatched leftover parts from the old plan"
        );

        let served_bytes = served.load(Ordering::SeqCst);
        assert_eq!(
            served_bytes,
            body.len() as u64 + 1,
            "expected a full fresh fetch (the old plan's parts must be discarded, not trusted): served {} of {} bytes",
            served_bytes,
            body.len()
        );
    }

    #[tokio::test]
    async fn test_resume_restarts_clean_when_foreign_download_reuses_output_path() {
        let body: Vec<u8> = (0..(12 * 1024 * 1024) as u32)
            .map(|i| (i % 256) as u8)
            .collect();
        let (base_url, served, _server) = spawn_ranged_media_server(body.clone()).await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let output_path = tmp.path().join("out.mp4");

        // Simulate a DIFFERENT, unrelated download that finished writing to
        // this exact output_path (same size, so it passes the existing
        // per-segment `existing_bytes <= total_size` guard) but is a
        // different resource entirely — its bytes must never end up in
        // this download's output.
        let foreign_body: Vec<u8> = vec![0xEE; body.len()];
        let segments = calculate_segments(body.len() as u64, 4, &output_path);
        for seg in &segments {
            let start = seg.start as usize;
            let end = seg.end as usize;
            write_stub_part(&seg.path, &foreign_body[start..=end]);
        }
        let foreign_identity = ResumeIdentity::new(
            "https://example.com/a-completely-different-video.mp4",
            body.len() as u64,
            4,
        );
        write_sidecar(&sidecar_path(&output_path), &foreign_identity)
            .await
            .expect("write sidecar");

        let engine = DownloadEngine::new(DownloadConfig {
            segments: 4,
            retry_attempts: 2,
            retry_delay: Duration::from_millis(5),
            request_delay: Duration::from_millis(1),
            ..Default::default()
        });

        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = engine.download(&base_url, &output_path, tx).await;
        assert!(
            result.is_ok(),
            "expected a clean restart to succeed: {:?}",
            result
        );

        let output = tokio::fs::read(&output_path).await.expect("read output");
        assert_eq!(
            output, body,
            "output must be the real download's bytes, not the foreign download's leftover parts"
        );
        assert_ne!(
            output, foreign_body,
            "sanity: output must not be the foreign body"
        );

        let served_bytes = served.load(Ordering::SeqCst);
        assert_eq!(
            served_bytes,
            body.len() as u64 + 1,
            "expected a full fresh fetch (the foreign parts must be discarded, not trusted)"
        );
    }

    #[tokio::test]
    async fn test_download_ignores_matching_parts_when_resume_disabled() {
        let body: Vec<u8> = (0..(12 * 1024 * 1024) as u32)
            .map(|i| (i % 256) as u8)
            .collect();
        let (base_url, served, _server) = spawn_ranged_media_server(body.clone()).await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let output_path = tmp.path().join("out.mp4");

        // Fully-correct, fully-matching parts + sidecar — the "everything
        // lines up" case that would normally resume for free (see the
        // matching-identity test above).
        let segments = calculate_segments(body.len() as u64, 4, &output_path);
        for seg in &segments {
            let start = seg.start as usize;
            let end = seg.end as usize;
            write_stub_part(&seg.path, &body[start..=end]);
        }
        let identity = ResumeIdentity::new(&base_url, body.len() as u64, 4);
        write_sidecar(&sidecar_path(&output_path), &identity)
            .await
            .expect("write sidecar");

        let engine = DownloadEngine::new(DownloadConfig {
            segments: 4,
            retry_attempts: 2,
            retry_delay: Duration::from_millis(5),
            request_delay: Duration::from_millis(1),
            enable_resume: false,
            ..Default::default()
        });

        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = engine.download(&base_url, &output_path, tx).await;
        assert!(
            result.is_ok(),
            "expected a full download to succeed: {:?}",
            result
        );

        let output = tokio::fs::read(&output_path).await.expect("read output");
        assert_eq!(output, body, "output must still be byte-correct");

        let served_bytes = served.load(Ordering::SeqCst);
        assert_eq!(
            served_bytes,
            body.len() as u64 + 1,
            "enable_resume=false must ignore even fully-matching parts and re-fetch everything: served {} of {} bytes",
            served_bytes,
            body.len()
        );
    }
}
