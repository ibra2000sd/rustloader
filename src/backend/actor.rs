use super::messages::{BackendCommand, BackendEvent};
use crate::database::{DatabaseManager, DownloadRecord};
use crate::downloader::{DownloadConfig, DownloadEngine};
use crate::extractor::{Extractor, Format, HybridExtractor, VideoInfo, YtDlpExtractor};
use crate::gui::DownloadProgressData;
use crate::queue::{DownloadTask, EventLog, QueueManager, TaskStatus};
use crate::utils::config::{AppSettings, OutputFormat, VideoQuality};
use crate::utils::{get_app_support_dir, FileOrganizer, MetadataManager, OrganizationSettings};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

/// Map a queue `TaskStatus` to the `downloads` table's `(status, completed_at,
/// error_message)` columns. The single source of truth for that mapping, used
/// both for the DB history writes and the GUI's `TaskStatusUpdated` event, so
/// the two never drift apart.
fn task_status_db_fields(status: &TaskStatus) -> (String, Option<DateTime<Utc>>, Option<String>) {
    match status {
        TaskStatus::Queued => ("Queued".to_string(), None, None),
        TaskStatus::Downloading => ("Downloading".to_string(), None, None),
        TaskStatus::Paused => ("Paused".to_string(), None, None),
        TaskStatus::Completed => ("Completed".to_string(), Some(Utc::now()), None),
        TaskStatus::Failed(e) => ("Failed".to_string(), Some(Utc::now()), Some(e.clone())),
        TaskStatus::Cancelled => ("Cancelled".to_string(), Some(Utc::now()), None),
    }
}

pub struct BackendActor {
    receiver: mpsc::Receiver<BackendCommand>,
    sender: mpsc::Sender<BackendEvent>,

    // Components
    extractor: Arc<HybridExtractor>,
    queue_manager: Arc<QueueManager>,
    db_manager: Arc<DatabaseManager>,
}

impl BackendActor {
    pub async fn new(
        settings: AppSettings,
        receiver: mpsc::Receiver<BackendCommand>,
        sender: mpsc::Sender<BackendEvent>,
        db_manager: Arc<DatabaseManager>,
    ) -> Result<Self> {
        // Cookie source from settings, applied to both extraction and download
        // so authenticated sites (e.g. YouTube) work from the GUI.
        let cookies = crate::utils::CookieConfig::new(
            settings.cookies_from_browser.clone(),
            settings.cookies_file.clone(),
        );

        // Initialize components
        // 1. Initialize Extractors
        let ytdlp = Arc::new(YtDlpExtractor::new()?.with_cookies(cookies.clone()));

        // 2. Build Hybrid Registry
        //
        // Empty native registry, same as the CLI (`cli.rs`). The native
        // YouTube extractor is an unimplemented stub; registering it made
        // every GUI YouTube download fail at `get_direct_url` (which routes
        // by `supports()` and, unlike `extract_info`, used to have no
        // fallback). Re-register it here once it actually extracts.
        let extractors: Vec<Arc<dyn Extractor>> = Vec::new();
        let fallback = ytdlp;
        let hybrid_extractor = Arc::new(HybridExtractor::new(extractors, fallback));
        let extractor = hybrid_extractor;

        let download_config = DownloadConfig {
            segments: settings.segments,
            connections_per_segment: 1,
            chunk_size: settings.chunk_size,
            retry_attempts: settings.retry_attempts,
            retry_delay: std::time::Duration::from_secs(2),
            enable_resume: settings.enable_resume,
            request_delay: std::time::Duration::from_millis(100),
        };

        let engine = DownloadEngine::new(download_config).with_ytdlp_options(
            crate::downloader::YtDlpOptions {
                cookies: cookies.clone(),
                ..Default::default()
            },
        );

        let org_settings = OrganizationSettings::default();
        let file_organizer = FileOrganizer::new(org_settings)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize file organizer: {}", e))?;

        let metadata_manager = MetadataManager::new(&file_organizer.base_dir);
        let file_organizer = Arc::new(file_organizer);
        let metadata_manager = Arc::new(metadata_manager);

        // 3. Initialize Event Log
        let app_support_dir = get_app_support_dir();
        let event_log = Arc::new(
            EventLog::new(&app_support_dir)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize event log: {}", e))?,
        );

        let queue_manager = Arc::new(QueueManager::new(
            settings.max_concurrent,
            engine,
            (*file_organizer).clone(),
            (*metadata_manager).clone(),
            event_log,
        ));

        Ok(Self {
            receiver,
            sender,
            extractor,
            queue_manager,
            db_manager,
        })
    }

    /// All persisted download history (all-time, including completed/failed/
    /// cancelled downloads that may no longer be in the live queue),
    /// most-recent-first. Nothing renders this yet — the GUI history list is
    /// a follow-up (Shape-3 PR-2) — but the data is live and durable now.
    pub async fn download_history(&self) -> Result<Vec<DownloadRecord>> {
        self.db_manager.get_all_downloads().await
    }

    pub async fn run(mut self) {
        info!("BackendActor started");

        // Rehydrate persistence state
        if let Err(e) = self.queue_manager.rehydrate().await {
            tracing::error!("Failed to rehydrate queue state: {}", e);
        }

        // Load persisted download history (Shape-3 PR-1). This is separate
        // from the queue rehydrate above: rehydrate reconstructs the LIVE
        // queue's runtime state from the EventLog; this reads the `downloads`
        // table, the durable history of every download ever started,
        // including ones long since cleared from the live queue. Nothing
        // consumes it yet (that's the GUI history list, PR-2) — this just
        // proves it's live and durable across restarts.
        match self.db_manager.get_all_downloads().await {
            Ok(history) => info!(
                "Loaded {} historical download record(s) from the downloads table",
                history.len()
            ),
            Err(e) => warn!("Failed to load download history: {}", e),
        }

        // Spawn Queue Processor (independent loop)
        let qm_clone = self.queue_manager.clone();
        tokio::spawn(async move {
            qm_clone.start().await;
        });

        // Spawn Monitor (polls queue and sends events)
        // In a future refactor, QueueManager should push events directly.
        // For now, we port the polling logic to keep changes scoped.
        let qm_monitor = self.queue_manager.clone();
        let sender_monitor = self.sender.clone();
        let db_monitor = Arc::clone(&self.db_manager);
        tokio::spawn(async move {
            Self::monitor_loop(qm_monitor, sender_monitor, db_monitor).await;
        });

        while let Some(cmd) = self.receiver.recv().await {
            match cmd {
                BackendCommand::ExtractInfo { url, cookies_file } => {
                    self.handle_extract_info(url, cookies_file).await;
                }
                BackendCommand::StartDownload {
                    video_info,
                    output_path,
                    format_id,
                    quality,
                    output_format,
                    insecure_tls,
                } => {
                    self.handle_start_download(
                        *video_info,
                        output_path,
                        format_id,
                        quality,
                        output_format,
                        insecure_tls,
                    )
                    .await;
                }
                BackendCommand::PauseDownload(id) => {
                    let _ = self.queue_manager.pause_task(&id).await;
                    // confirmation sent via monitor loop
                }
                BackendCommand::ResumeDownload(id) => {
                    let _ = self.queue_manager.resume_task(&id).await;
                }
                BackendCommand::CancelDownload(id) => {
                    let _ = self.queue_manager.cancel_task(&id).await;
                }
                BackendCommand::RemoveTask(id) => {
                    let _ = self.queue_manager.remove_task(&id).await;
                }
                BackendCommand::ClearCompleted => {
                    let _ = self.queue_manager.clear_completed().await;
                }
                BackendCommand::ResumeAll => {
                    let _ = self.queue_manager.resume_all().await;
                }
                BackendCommand::Shutdown => {
                    info!("BackendActor shutting down");
                    break;
                }
            }
        }
    }

    async fn handle_extract_info(&self, url: String, cookies_file: Option<PathBuf>) {
        let _ = self.sender.send(BackendEvent::ExtractionStarted).await;

        // A per-request cookies file (browser-bridge, F-EXT-001) gets a
        // one-off extractor carrying exactly those cookies; everything else
        // uses the settings-derived extractor built at startup. Construction
        // is cheap (a yt-dlp path probe); the cookie flags still come only
        // from `CookieConfig` (I-7).
        let extractor: Arc<HybridExtractor> = match cookies_file {
            Some(file) => match YtDlpExtractor::new() {
                Ok(ytdlp) => {
                    let cookies = crate::utils::CookieConfig::new(None, Some(file));
                    Arc::new(HybridExtractor::new(
                        Vec::new(),
                        Arc::new(ytdlp.with_cookies(cookies)),
                    ))
                }
                Err(e) => {
                    warn!("bridge extraction: yt-dlp unavailable ({e}); using default extractor");
                    self.extractor.clone()
                }
            },
            None => self.extractor.clone(),
        };

        match extractor.extract_info(&url).await {
            Ok(info) => {
                let _ = self
                    .sender
                    .send(BackendEvent::ExtractionCompleted(Ok(info)))
                    .await;
            }
            Err(e) => {
                // Log it: the error otherwise only surfaces to the GUI, so a
                // non-timeout failure would be invisible to anyone reading the logs.
                warn!("Extraction failed for {}: {}", url, e);
                let _ = self
                    .sender
                    .send(BackendEvent::ExtractionCompleted(Err(e.to_string())))
                    .await;
            }
        }
    }

    async fn handle_start_download(
        &self,
        video_info: VideoInfo,
        output_path: PathBuf,
        format_id: Option<String>,
        quality: VideoQuality,
        output_format: OutputFormat,
        insecure_tls: bool,
    ) {
        // Validation and setup logic ported from BackendBridge

        // 1. Format Selection
        let format = match Self::select_format(&video_info, format_id, &quality) {
            Ok(f) => f,
            Err(e) => {
                // Log it: the error otherwise only surfaces to the GUI status
                // bar, so a failed download start would be invisible to anyone
                // reading the logs.
                warn!("Format selection failed for {}: {}", video_info.url, e);
                let _ = self.sender.send(BackendEvent::Error(e)).await;
                return;
            }
        };

        // 2. Get Direct URL
        let download_url = match self
            .get_download_url(&video_info, &format, &output_format)
            .await
        {
            Ok(url) => url,
            Err(e) => {
                // Log it: same reasoning as the format-selection arm above.
                warn!("Direct URL resolution failed for {}: {}", video_info.url, e);
                let _ = self.sender.send(BackendEvent::Error(e)).await;
                return;
            }
        };

        // 3. Create Task
        let task_id = Uuid::new_v4().to_string();
        let mut updated_format = format.clone();
        updated_format.url = download_url;

        let task = DownloadTask {
            id: task_id.clone(),
            video_info: video_info.clone(),
            output_path: output_path.clone(), // Note: caller should handle path logic? Or we duplicate it here?
            // The original code did path fixing here. Let's do a basic fix if needed.
            format: updated_format,
            status: TaskStatus::Queued,
            progress: None,
            added_at: Utc::now(),
            output_format,
            insecure_tls,
        };

        // 4. Add to Queue
        if let Err(e) = self.queue_manager.add_task(task).await {
            let _ = self
                .sender
                .send(BackendEvent::DownloadFailed {
                    task_id,
                    error: e.to_string(),
                })
                .await;
            return;
        }

        // Persist the initial history row now that the task is actually
        // queued. Best-effort: a write failure here must not fail the
        // download — the queue (added above) is already the runtime source
        // of truth for this task; this is a durable log of it, not a second
        // one.
        let record = DownloadRecord {
            id: task_id.clone(),
            url: video_info.url.clone(),
            title: video_info.title.clone(),
            output_path: output_path.clone(),
            file_size: format.filesize,
            status: "Queued".to_string(),
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
        };
        if let Err(e) = self.db_manager.save_download(&record).await {
            warn!("Failed to persist download history for {}: {}", task_id, e);
        }

        let _ = self
            .sender
            .send(BackendEvent::DownloadStarted {
                task_id,
                video_info,
            })
            .await;
    }

    /// Choose the format to download.
    ///
    /// With an explicit `format_id`, returns that exact format. Otherwise the
    /// user's `quality` choice decides (B-GUI-003 — it used to be ignored and
    /// the maximum resolution always won):
    ///
    /// - `Best`: the highest-resolution format across ALL formats — DASH-split
    ///   video included — i.e. `Specific` with no height cap, mirroring
    ///   yt-dlp's default `bv*+ba/b`. (It used to prefer the best progressive
    ///   format, but YouTube offers no progressive above 360p, so "Best
    ///   Available" delivered WORSE quality than picking 1080p.) A video-only
    ///   pick is made sound by the queue's `PageFallback` exactly as for
    ///   `Specific`. At equal resolution a progressive format still wins, and
    ///   sources with no resolution data (direct files, audio-only) remain
    ///   pickable — never an error where the old `Best` succeeded.
    /// - `Specific(h)`: the best format with `height <= h` across ALL formats
    ///   (DASH-split video included — a video-only pick is made sound by the
    ///   queue's `PageFallback`, which appends `+bestaudio` to the yt-dlp
    ///   `-f` spec it rebuilds from the chosen id). If nothing fits under the
    ///   cap, the nearest format above it (smallest height) — never an error
    ///   where `Best` would have succeeded.
    /// - `Worst`: the smallest format that still has a video track.
    ///
    /// Either way the choice travels as a concrete format: a complete format's
    /// resolved URL feeds the native engine, while a video-only pick travels
    /// as the page URL + its id so the yt-dlp path can merge in audio (see
    /// [`Self::get_download_url`]).
    fn select_format(
        video_info: &VideoInfo,
        format_id: Option<String>,
        quality: &VideoQuality,
    ) -> Result<Format, String> {
        // Logic from BackendBridge::start_download
        if let Some(id) = format_id {
            return video_info
                .formats
                .iter()
                .find(|f| f.format_id == id)
                .cloned()
                .ok_or_else(|| "Format not found".to_string());
        }

        fn has_video(f: &Format) -> bool {
            f.vcodec.as_deref().unwrap_or("none") != "none"
        }
        fn has_audio(f: &Format) -> bool {
            f.acodec.as_deref().unwrap_or("none") != "none"
        }
        fn area(f: &Format) -> u64 {
            f.width.unwrap_or(0) as u64 * f.height.unwrap_or(0) as u64
        }

        // Storyboards are never downloadable content.
        let pool: Vec<&Format> = video_info
            .formats
            .iter()
            .filter(|f| !f.format_id.starts_with("sb"))
            .collect();

        let picked: Option<&Format> = match quality {
            VideoQuality::Specific(h) if h.parse::<u32>().is_ok() => {
                let cap: u32 = h.parse().expect("guarded by the match arm");
                pool.iter()
                    .copied()
                    .filter(|f| f.height.unwrap_or(0) <= cap)
                    // Prefer a progressive format over a video-only one of the
                    // same resolution; area decides otherwise.
                    .max_by_key(|f| (area(f), has_audio(f) && has_video(f)))
                    .or_else(|| {
                        // Nothing at or below the cap: nearest above it.
                        pool.iter()
                            .copied()
                            .filter(|f| has_video(f))
                            .min_by_key(|f| f.height.unwrap_or(u32::MAX))
                    })
            }
            VideoQuality::Worst => pool
                .iter()
                .copied()
                .filter(|f| has_video(f))
                .min_by_key(|f| (area(f), !has_audio(f))),
            // `Best`, and `Specific` values that aren't a pixel height (the
            // GUI only produces numeric ones): the true best — the highest
            // resolution across ALL formats, the same selection as
            // `Specific` with no height cap. A video-only winner (YouTube's
            // DASH ladder) gets audio from `PageFallback`'s `+bestaudio`;
            // formats without resolution data (direct files, audio-only)
            // have area 0 and are still picked when they are all there is.
            _ => pool
                .iter()
                .copied()
                .max_by_key(|f| (area(f), has_audio(f) && has_video(f))),
        };

        picked
            .or_else(|| video_info.formats.first())
            .cloned()
            .ok_or_else(|| "No downloadable format found".to_string())
    }

    async fn get_download_url(
        &self,
        video_info: &VideoInfo,
        format: &Format,
        output_format: &OutputFormat,
    ) -> Result<String, String> {
        // A video-only pick (DASH-split video, acodec "none") can never be
        // made whole by the native path — its direct URL serves exactly one,
        // silent, stream, and YouTube's IP-bound URLs probe fine so the
        // engine happily downloads it. Hand back the page URL instead (the
        // same move as the HLS check below): the engine's probe then routes
        // to yt-dlp, where the queue's `PageFallback` spec
        // (`{id}+bestaudio/{id}/best`) downloads video + audio and ffmpeg
        // merges them.
        let video_only = format.vcodec.as_deref().unwrap_or("none") != "none"
            && format.acodec.as_deref().unwrap_or("none") == "none";
        if video_only && !video_info.url.is_empty() {
            return Ok(video_info.url.clone());
        }

        // An explicit output format (B-GUI-005) needs ffmpeg post-processing
        // (remux or audio extraction), which only the yt-dlp path performs —
        // the native engine would download the direct URL bytes and skip the
        // conversion entirely. Same page-URL move as the video-only branch:
        // the engine's probe sees HTML and routes to yt-dlp.
        if !output_format.is_best() && !video_info.url.is_empty() {
            return Ok(video_info.url.clone());
        }

        let direct_url = self
            .extractor
            .get_direct_url(&video_info.url, &format.format_id)
            .await
            .map_err(|e| e.to_string())?;

        // HLS Check
        if direct_url.contains(".m3u8") || direct_url.contains("/manifest") {
            Ok(video_info.url.clone())
        } else {
            Ok(direct_url)
        }
    }

    async fn monitor_loop(
        qm: Arc<QueueManager>,
        sender: mpsc::Sender<BackendEvent>,
        db_manager: Arc<DatabaseManager>,
    ) {
        let mut last_statuses = std::collections::HashMap::new();

        // Seed from the rehydrated queue (`rehydrate` has already run by the
        // time this is spawned). Those statuses are history, not transitions
        // that just happened: without the baseline, the first poll diffs every
        // rehydrated task against the `Queued` default and re-fires
        // DownloadCompleted/DownloadFailed for downloads that finished
        // sessions ago — announcing them in the status bar and, worse,
        // rewriting each history row's `completed_at` with the launch time on
        // every start. Tasks created later are deliberately NOT seeded, so
        // their real transitions still fire, however fast they arrive.
        for task in qm.get_all_tasks().await {
            last_statuses.insert(task.id.clone(), task.status.clone());
        }

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Faster updates?

            let tasks = qm.get_all_tasks().await;

            for task in tasks {
                // Check status change
                let last_status = last_statuses
                    .get(&task.id)
                    .cloned()
                    .unwrap_or(TaskStatus::Queued);

                if task.status != last_status {
                    let (status_str, completed_at, error_message) =
                        task_status_db_fields(&task.status);

                    let _ = sender
                        .send(BackendEvent::TaskStatusUpdated {
                            task_id: task.id.clone(),
                            status: status_str.clone(),
                        })
                        .await;

                    if matches!(task.status, TaskStatus::Completed) {
                        let _ = sender
                            .send(BackendEvent::DownloadCompleted {
                                task_id: task.id.clone(),
                                file_path: Some(task.output_path.to_string_lossy().to_string()),
                            })
                            .await;
                    }
                    if let TaskStatus::Failed(e) = &task.status {
                        let _ = sender
                            .send(BackendEvent::DownloadFailed {
                                task_id: task.id.clone(),
                                error: e.clone(),
                            })
                            .await;
                    }

                    // Persist the transition to the `downloads` history table.
                    // Best-effort: the live queue (via `last_statuses`/`qm`)
                    // remains the runtime authority for this task's state
                    // regardless of whether this write succeeds — a failure
                    // here must never affect the download itself.
                    let record = DownloadRecord {
                        id: task.id.clone(),
                        url: task.video_info.url.clone(),
                        title: task.video_info.title.clone(),
                        output_path: task.output_path.clone(),
                        file_size: task.format.filesize,
                        status: status_str,
                        created_at: task.added_at,
                        completed_at,
                        error_message,
                    };
                    if let Err(e) = db_manager.save_download(&record).await {
                        warn!(
                            "Failed to persist download history update for {}: {}",
                            task.id, e
                        );
                    }

                    last_statuses.insert(task.id.clone(), task.status.clone());
                }

                // Send Progress only while the task is actually active — never
                // for terminal states (Completed/Failed/Cancelled), otherwise a
                // late progress event would flip a finished row back to
                // "Downloading" in the GUI.
                let active = matches!(
                    task.status,
                    TaskStatus::Downloading | TaskStatus::Queued | TaskStatus::Paused
                );
                if active {
                    if let Some(progress) = &task.progress {
                        let data = DownloadProgressData {
                            progress: progress.percentage() as f32,
                            speed: progress.speed,
                            downloaded: progress.downloaded_bytes,
                            total: progress.total_bytes,
                            eta: progress.eta.map(|d| d.as_secs()),
                        };

                        let _ = sender
                            .send(BackendEvent::DownloadProgress {
                                task_id: task.id.clone(),
                                data,
                            })
                            .await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_status_db_fields_sets_completed_at_only_for_terminal_states() {
        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Queued);
        assert_eq!(status, "Queued");
        assert!(completed_at.is_none());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Downloading);
        assert_eq!(status, "Downloading");
        assert!(completed_at.is_none());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Paused);
        assert_eq!(status, "Paused");
        assert!(completed_at.is_none());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Completed);
        assert_eq!(status, "Completed");
        assert!(completed_at.is_some());
        assert!(error.is_none());

        let (status, completed_at, error) = task_status_db_fields(&TaskStatus::Cancelled);
        assert_eq!(status, "Cancelled");
        assert!(completed_at.is_some());
        assert!(error.is_none());
    }

    #[test]
    fn task_status_db_fields_carries_the_failure_message() {
        let (status, completed_at, error) =
            task_status_db_fields(&TaskStatus::Failed("boom".to_string()));
        assert_eq!(status, "Failed");
        assert!(completed_at.is_some());
        assert_eq!(error.as_deref(), Some("boom"));
    }

    fn fmt(id: &str, vcodec: Option<&str>, acodec: Option<&str>, w: u32, h: u32) -> Format {
        Format {
            format_id: id.to_string(),
            vcodec: vcodec.map(str::to_string),
            acodec: acodec.map(str::to_string),
            width: Some(w),
            height: Some(h),
            url: format!("https://example.com/{id}"),
            ..Default::default()
        }
    }

    fn info(formats: Vec<Format>) -> VideoInfo {
        VideoInfo {
            title: "t".to_string(),
            url: "https://example.com/page".to_string(),
            formats,
            ..Default::default()
        }
    }

    #[test]
    fn explicit_format_id_is_returned() {
        let vi = info(vec![fmt("18", Some("h264"), Some("aac"), 640, 360)]);
        let f =
            BackendActor::select_format(&vi, Some("18".to_string()), &VideoQuality::Best).unwrap();
        assert_eq!(f.format_id, "18");
    }

    #[test]
    fn best_picks_highest_resolution_across_all_formats() {
        // Deliberate behaviour change: Best used to prefer the best
        // *progressive* format ("high", 1080p), but YouTube offers no
        // progressive above 360p, so that made "Best Available" worse than
        // Specific(1080). Best now means TRUE best: the highest resolution,
        // video-only DASH included (PageFallback adds `+bestaudio`).
        let vi = info(vec![
            fmt("low", Some("h264"), Some("aac"), 640, 360),
            fmt("high", Some("h264"), Some("aac"), 1920, 1080),
            fmt("videoonly", Some("vp9"), Some("none"), 3840, 2160),
        ]);
        let f = BackendActor::select_format(&vi, None, &VideoQuality::Best).unwrap();
        assert_eq!(f.format_id, "videoonly"); // the 4k, not the 1080p progressive
    }

    #[test]
    fn best_on_youtube_shaped_ladder_picks_top_dash_rung() {
        // The B-GUI-00x regression itself: on a YouTube-shaped list Best used
        // to pick "18" (360p progressive); it must pick the 1080p DASH rung.
        let f = BackendActor::select_format(&youtube_like(), None, &VideoQuality::Best).unwrap();
        assert_eq!(f.format_id, "137");
    }

    #[test]
    fn best_prefers_progressive_at_equal_resolution() {
        // No pointless DASH split where a complete format is just as good.
        let vi = info(vec![
            fmt("videoonly", Some("h264"), Some("none"), 1280, 720),
            fmt("progressive", Some("h264"), Some("aac"), 1280, 720),
        ]);
        let f = BackendActor::select_format(&vi, None, &VideoQuality::Best).unwrap();
        assert_eq!(f.format_id, "progressive");
    }

    #[test]
    fn direct_file_single_format_no_codecs_now_selectable() {
        // A direct .mp4 yields one format with no codec info — previously this
        // returned Err("No combined format found") and the GUI never downloaded.
        let vi = info(vec![Format {
            format_id: "0".to_string(),
            url: "https://example.com/video.mp4".to_string(),
            ..Default::default()
        }]);
        let f = BackendActor::select_format(&vi, None, &VideoQuality::Best)
            .expect("must pick a format, not error");
        assert_eq!(f.format_id, "0");
    }

    #[test]
    fn dash_split_picks_the_video_track_not_an_error() {
        // No single progressive format (video-only + audio-only). Must pick
        // the video track rather than fail (never-fail property).
        let vi = info(vec![
            fmt("video", Some("vp9"), Some("none"), 1920, 1080),
            fmt("audio", Some("none"), Some("mp4a"), 0, 0),
        ]);
        let f = BackendActor::select_format(&vi, None, &VideoQuality::Best)
            .expect("must pick a format, not error");
        assert_eq!(f.format_id, "video"); // best by resolution
    }

    #[test]
    fn empty_formats_is_an_error() {
        let vi = info(vec![]);
        assert!(BackendActor::select_format(&vi, None, &VideoQuality::Best).is_err());
    }

    // B-GUI-003 — the quality choice must constrain the selection.

    fn specific(h: &str) -> VideoQuality {
        VideoQuality::Specific(h.to_string())
    }

    /// A YouTube-shaped format list: one low progressive format plus a DASH
    /// ladder of video-only formats and an audio-only track.
    fn youtube_like() -> VideoInfo {
        info(vec![
            fmt("18", Some("h264"), Some("aac"), 640, 360),
            fmt("135", Some("h264"), Some("none"), 854, 480),
            fmt("136", Some("h264"), Some("none"), 1280, 720),
            fmt("137", Some("h264"), Some("none"), 1920, 1080),
            fmt("audio", Some("none"), Some("mp4a"), 0, 0),
        ])
    }

    #[test]
    fn specific_height_picks_best_format_within_cap() {
        let vi = youtube_like();
        for (cap, expected) in [("480", "135"), ("720", "136"), ("1080", "137")] {
            let f = BackendActor::select_format(&vi, None, &specific(cap)).unwrap();
            assert_eq!(f.format_id, expected, "cap {cap} must pick {expected}");
        }
    }

    #[test]
    fn specific_height_prefers_progressive_at_equal_resolution() {
        let vi = info(vec![
            fmt("progressive", Some("h264"), Some("aac"), 854, 480),
            fmt("videoonly", Some("h264"), Some("none"), 854, 480),
        ]);
        let f = BackendActor::select_format(&vi, None, &specific("480")).unwrap();
        assert_eq!(f.format_id, "progressive");
    }

    #[test]
    fn specific_height_below_everything_picks_nearest_above() {
        let vi = info(vec![
            fmt("720", Some("h264"), Some("aac"), 1280, 720),
            fmt("1080", Some("h264"), Some("aac"), 1920, 1080),
        ]);
        let f = BackendActor::select_format(&vi, None, &specific("480")).unwrap();
        assert_eq!(f.format_id, "720"); // nothing <= 480: nearest above, not max
    }

    #[test]
    fn specific_height_still_selects_a_heightless_direct_file() {
        // Direct files carry no height; the cap must not make them unreachable.
        let vi = info(vec![Format {
            format_id: "0".to_string(),
            url: "https://example.com/video.mp4".to_string(),
            ..Default::default()
        }]);
        let f = BackendActor::select_format(&vi, None, &specific("480"))
            .expect("must pick a format, not error");
        assert_eq!(f.format_id, "0");
    }

    #[test]
    fn non_numeric_specific_behaves_like_best() {
        let vi = youtube_like();
        let f = BackendActor::select_format(&vi, None, &specific("Custom")).unwrap();
        assert_eq!(f.format_id, "137"); // same pick as Best: the true best
    }

    #[test]
    fn worst_picks_smallest_video_format_not_audio() {
        let vi = youtube_like();
        let f = BackendActor::select_format(&vi, None, &VideoQuality::Worst).unwrap();
        assert_eq!(f.format_id, "18"); // smallest with a video track, not "audio"
    }

    #[test]
    fn explicit_format_id_outranks_quality() {
        let vi = youtube_like();
        let f =
            BackendActor::select_format(&vi, Some("137".to_string()), &specific("480")).unwrap();
        assert_eq!(f.format_id, "137");
    }

    // ============================================================
    // MONITOR BASELINE ON RESTART
    // `last_statuses` started empty, so the first poll diffed every
    // rehydrated task against the `Queued` default and treated a
    // download that finished sessions ago as a transition that had
    // just happened.
    // ============================================================

    #[tokio::test]
    async fn rehydrated_terminal_tasks_neither_refire_events_nor_rewrite_history() {
        use crate::database::initialize_database;
        use crate::queue::QueueEvent;

        let tmp = tempfile::tempdir().expect("tempdir");
        let task_id = "finished-last-week";
        let output_path = tmp.path().join("video.mp4");

        // A download that completed in an earlier session, as the event log
        // records it.
        let event_log = Arc::new(EventLog::new(tmp.path()).await.expect("event log"));
        event_log
            .log(QueueEvent::TaskAdded {
                task_id: task_id.to_string(),
                video_info: Box::new(VideoInfo {
                    title: "old download".to_string(),
                    url: "https://example.com/page".to_string(),
                    ..Default::default()
                }),
                format: Box::new(Format::default()),
                output_path: output_path.clone(),
                timestamp: Utc::now(),
                output_format: OutputFormat::Best,
                insecure_tls: false,
            })
            .await
            .expect("log TaskAdded");
        event_log
            .log(QueueEvent::TaskCompleted {
                task_id: task_id.to_string(),
                output_path: output_path.clone(),
                timestamp: Utc::now(),
            })
            .await
            .expect("log TaskCompleted");

        let organizer = FileOrganizer::new(OrganizationSettings::default())
            .await
            .expect("file organizer");
        let queue_manager = Arc::new(QueueManager::new(
            1,
            DownloadEngine::new(DownloadConfig::default()),
            organizer,
            MetadataManager::new(tmp.path()),
            Arc::clone(&event_log),
        ));
        queue_manager.rehydrate().await.expect("rehydrate");

        // Its history row, carrying the real completion time.
        let db_path = tmp.path().join("history.db");
        let pool = initialize_database(&format!("sqlite://{}?mode=rwc", db_path.display()))
            .await
            .expect("init db");
        let db_manager = Arc::new(DatabaseManager::new(pool));
        let finished_at = Utc::now() - chrono::Duration::days(7);
        db_manager
            .save_download(&DownloadRecord {
                id: task_id.to_string(),
                url: "https://example.com/page".to_string(),
                title: "old download".to_string(),
                output_path: output_path.clone(),
                file_size: Some(1_000_000),
                status: "Completed".to_string(),
                created_at: finished_at,
                completed_at: Some(finished_at),
                error_message: None,
            })
            .await
            .expect("seed history row");

        let (tx, mut rx) = mpsc::channel(64);
        let monitor = tokio::spawn(BackendActor::monitor_loop(
            Arc::clone(&queue_manager),
            tx,
            Arc::clone(&db_manager),
        ));

        // Several 100ms poll ticks.
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        monitor.abort();

        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, BackendEvent::DownloadCompleted { .. })),
            "a download that finished in an earlier session must not be announced again, got {} event(s)",
            events.len()
        );

        let history = db_manager
            .get_all_downloads()
            .await
            .expect("get_all_downloads");
        let row = history
            .iter()
            .find(|r| r.id == task_id)
            .expect("history row present");
        assert_eq!(
            row.completed_at.map(|t| t.timestamp()),
            Some(finished_at.timestamp()),
            "the recorded completion time must survive a restart, not become the launch time"
        );
    }
}
