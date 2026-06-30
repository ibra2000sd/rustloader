# Architecture

Verified against `src/` at main `1c038e2` (v0.8.1). `file:line` anchors will
drift as code changes — re-confirm before relying on a specific line. Stage
history (v0.1–v0.5) is summarized from the project's own `ROADMAP.md`; the
current-code facts below were read directly.

## Module layout (`src/lib.rs`)

```
app        backend     cli        database   downloader
extractor  gui         queue      utils
```

- **`main.rs`** — entry point. Parses `Cli` (`src/cli.rs`); if `cli.is_cli_mode()`
  it runs the CLI on a temporary Tokio runtime and exits, otherwise it launches
  the Iced GUI (`gui::RustloaderApp::run`).
- **`gui/`** — Iced 0.12 desktop UI: `app.rs` (state + update loop + the
  subscription that drains backend events), `views/`, `components/`, `theme.rs`,
  `clipboard.rs`, `icon.rs`.
- **`backend/`** — the actor that decouples the GUI from the work.
- **`extractor/`** — turns a URL into `VideoInfo`/`Format`s.
- **`downloader/`** — the engine that turns a `Format` into a file on disk.
- **`queue/`** — the task FSM, scheduler, and event log (single source of truth
  for task state).
- **`database/`** — SQLite persistence.
- **`utils/`** — cookies, config, platform/bundle paths, dependency checks,
  file organization, metadata, errors.

## The actor model (`backend/`)

The GUI never touches the engine directly. It sends **`BackendCommand`**
(`backend/messages.rs`) — `ExtractInfo`, `StartDownload{…}`, `PauseDownload(id)`,
`ResumeDownload(id)`, `CancelDownload(id)`, `ResumeAll`, … — over an `mpsc`
channel to the `BackendActor` (`backend/actor.rs`). The actor owns the
`HybridExtractor`, the `DownloadEngine`, the `QueueManager`, and the database, and
emits **`BackendEvent`** back to the GUI — `DownloadStarted`, `DownloadProgress
{ data }`, `DownloadCompleted`, `DownloadFailed`, and `TaskStatusUpdate { status:
"Paused"|"Cancelled"|"Queued" }`. The GUI consumes these via an Iced
subscription. (Roadmap stage v0.2.)

## Extraction pipeline (`extractor/`)

`HybridExtractor` (`extractor/hybrid.rs`) tries registered extractors in order and
falls back to yt-dlp:

- **`NativeYoutubeExtractor`** (`extractor/native/`) — first-choice for YouTube.
- **`YtDlpExtractor`** (`extractor/ytdlp.rs`) — the universal fallback. It shells
  out to the `yt-dlp` binary (located via macOS `.app` bundle → system `PATH` →
  common paths, `find_ytdlp()`). All four extraction calls (`extract_info`,
  `extract_playlist`, `search`, `get_direct_url`) run through a single bounded
  helper `run_ytdlp_bounded` → `run_bounded(cmd, EXTRACTION_TIMEOUT=60s)`, which
  sets `kill_on_drop(true)` and wraps `cmd.output()` in `tokio::time::timeout`
  (added in PR #23 — see invariants). Cookie flags from a shared `CookieConfig`
  are prepended to every call.

## Download pipeline (`downloader/engine.rs`)

`DownloadEngine::download()` routes by a single ranged probe GET
(`Range: bytes=0-0`, 10s timeout) that returns `(supports_ranges, size,
content_type)`:

1. **Non-direct-media `Content-Type`** → `download_via_ytdlp` (the yt-dlp
   download path; HLS/DASH/HTML-hosted media). Bounded by
   `timeout(1800s, child.wait())` + `child.kill()`. yt-dlp progress is parsed
   from stderr. (A defensive guard refuses to save a non-media response as a
   media file — PR #14.)
2. **Direct media, no range support or `< 1 MB`** → `download_simple` (single
   stream).
3. **Direct media with range support** → **native segmented engine**:
   `calculate_segments()` splits the byte range; segments download in parallel
   (`buffer_unordered(config.segments)`), each issuing `Range: bytes=start-end`
   and writing a `.partN` file; an aggregator emits `DownloadProgress` ~every 1s;
   a **stall watchdog** classifies no-forward-progress as `DownloadStatus::Stalled`
   (notify-only, no recovery); on success `merge_segments` concatenates and
   `cleanup_segments` deletes the parts.

**Progress contract** (`downloader/progress.rs`): `DownloadProgress { total_bytes,
downloaded_bytes, speed: f64, eta: Option<Duration>, status, segments_completed,
total_segments }`. `DownloadStatus = Initializing | Downloading | Merging |
Completed | Failed(String) | Paused | Stalled`.

## Queue / task FSM (`queue/`)

`QueueManager` (`queue/manager.rs`) is the **single source of truth** for task
state and the scheduler (`process_queue()` enforces max-concurrency over an
`active_downloads` map). `TaskStatus = Queued | Downloading | Paused | Completed |
Failed | Cancelled`. (Roadmap stages v0.4–v0.5.)

- **pause** = abort the in-flight join handle + mark `Paused` (no byte-resume
  today — see invariants & backlog).
- **resume** = set status `Queued` → `process_queue()` runs the download **again
  from the start**.
- **cancel** = abort + mark `Cancelled`.

**Event sourcing** (`queue/events.rs`, roadmap v0.3): every transition appends to
an `EventLog` (JSONL, append-only); `rehydrate()` reconstructs queue state from
the log on startup (malformed lines skipped); previously-`Started` tasks rehydrate
as `Paused` to avoid an auto-blast on restart.

**Locking hierarchy** (documented, roadmap v0.4): queue lock (L2) is acquired
before the `active_downloads` lock (L1). Preserve this order to avoid deadlock.

## Persistence (`database/`)

SQLite via `sqlx`. `initialize_database()` creates three tables
(`database/schema.rs`): **`downloads`**, **`download_segments`**, **`settings`**.
`database/operations.rs` reads/writes `download_segments`
(`INSERT OR REPLACE` / `SELECT … ORDER BY segment_number` / `DELETE … WHERE
download_id`). NOTE: the relationship between this `download_segments` persistence
and the engine's in-memory segment progress is not fully traced — relevant to the
resume work (see backlog `F-DL-003`).

## Cross-cutting

- **`utils/cookies.rs` — `CookieConfig`**: single source of truth for yt-dlp
  cookie args (`--cookies-from-browser <b>` and/or `--cookies <file>`), applied to
  **both** extraction and download. `default()` is empty and emits nothing.
- **`utils/config.rs` / settings table**: user settings (incl. the currently
  **dead** `enable_resume` flag — see backlog).
- **`utils/platform.rs`, `utils/bundle_paths.rs`, `utils/depcheck.rs`**: locate
  bundled/system binaries (yt-dlp, ffprobe) per OS; this is the precedent any
  future external-binary integration (e.g. aria2) must mirror.
- **`utils/organizer.rs`, `utils/metadata.rs`**: quality-based file organization.
- **`utils/error.rs` — `RustloaderError`**: `YtDlpNotFound`, `ExtractionError`,
  `DownloadError`, `NetworkError`, `IoError`, `DatabaseError`, `SerializationError`,
  `InvalidUrl`, `TaskNotFound`, `OperationFailed`.

## Build / CI

CI matrix (ubuntu/macOS/windows): `cargo test --all`, `cargo clippy
--all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`,
`cargo audit`. Release workflow builds per-target binaries; macOS bundles yt-dlp
inside the `.app`.
