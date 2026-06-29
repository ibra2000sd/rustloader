//! Command-line interface for headless downloads.
//!
//! The CLI is a thin front-end over Rustloader's existing engine: it parses
//! ergonomic flags (ported from the legacy `rustloader2` CLI surface,
//! `src/cli.rs` in that repo) and drives the same [`DownloadEngine`] /
//! [`HybridExtractor`] the GUI uses. It deliberately does **not** implement its
//! own download logic — every flag maps onto options consumed by the existing
//! engine yt-dlp path (see [`crate::downloader::build_ytdlp_args`]).
//!
//! With no URL the binary launches the GUI (see `main.rs`); when a URL is
//! supplied it runs a single headless download.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;

use crate::downloader::{build_ytdlp_args, DownloadEngine, DownloadProgress, YtDlpOptions};
use crate::extractor::{HybridExtractor, YtDlpExtractor};
use crate::utils;

/// Rustloader entry arguments (GUI by default, CLI when a URL is given).
#[derive(Parser, Debug, Default)]
#[command(
    name = "rustloader",
    version,
    about = "High-performance video downloader (GUI by default; CLI when a URL is given)"
)]
pub struct Cli {
    /// URL of the video to download. If omitted, the GUI is launched.
    pub url: Option<String>,

    /// Maximum video quality / height.
    #[arg(short = 'q', long, value_parser = ["480", "720", "1080"])]
    pub quality: Option<String>,

    /// Output type: `mp4` (video, default) or `mp3` (audio only).
    #[arg(short = 'f', long, value_parser = ["mp4", "mp3"])]
    pub format: Option<String>,

    /// Start time of a clip section (e.g. 00:01:00).
    #[arg(short = 's', long)]
    pub start_time: Option<String>,

    /// End time of a clip section (e.g. 00:02:00).
    #[arg(short = 'e', long)]
    pub end_time: Option<String>,

    /// Download subtitles if available.
    #[arg(long = "subs")]
    pub subs: bool,

    /// Download the entire playlist instead of a single video.
    #[arg(short = 'p', long)]
    pub playlist: bool,

    /// Directory to save into (defaults to the system Downloads folder).
    #[arg(short = 'o', long)]
    pub output_dir: Option<PathBuf>,

    /// Audio bitrate for extracted audio (e.g. 128K).
    #[arg(long)]
    pub bitrate: Option<String>,

    /// Print the resolved download plan (engine + yt-dlp args) and exit without
    /// downloading. Useful for scripting and tests; performs no network I/O.
    #[arg(long)]
    pub dry_run: bool,

    /// Legacy alias for a single headless download (kept for backwards compat).
    #[arg(long, hide = true)]
    pub test_download: Option<String>,

    /// Read cookies from this browser for sites that require authentication
    /// (e.g. YouTube's "Sign in to confirm you're not a bot"). One of:
    /// chrome, firefox, safari, edge, brave, chromium, opera, vivaldi.
    #[arg(long = "cookies-from-browser", value_name = "BROWSER")]
    pub cookies_from_browser: Option<String>,

    /// Path to a Netscape-format cookies.txt file passed to yt-dlp via
    /// `--cookies`.
    #[arg(long = "cookies", value_name = "FILE")]
    pub cookies_file: Option<PathBuf>,
}

impl Cli {
    /// The URL to download, if the binary was invoked in CLI mode.
    pub fn target_url(&self) -> Option<&str> {
        self.url.as_deref().or(self.test_download.as_deref())
    }

    /// Cookie configuration derived from `--cookies-from-browser` / `--cookies`.
    pub fn cookie_config(&self) -> crate::utils::CookieConfig {
        crate::utils::CookieConfig::new(
            self.cookies_from_browser.clone(),
            self.cookies_file.clone(),
        )
    }

    /// True when the binary should run a headless download rather than the GUI.
    pub fn is_cli_mode(&self) -> bool {
        self.target_url().is_some()
    }

    /// Translate the parsed flags into engine-level yt-dlp options.
    pub fn to_ytdlp_options(&self) -> YtDlpOptions {
        let audio_only = self.format.as_deref() == Some("mp3");
        YtDlpOptions {
            quality: self.quality.as_deref().and_then(|q| q.parse::<u32>().ok()),
            audio_only,
            audio_format: if audio_only {
                Some("mp3".to_string())
            } else {
                None
            },
            subtitles: self.subs,
            playlist: self.playlist,
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            audio_bitrate: self.bitrate.clone(),
            cookies: self.cookie_config(),
        }
    }

    /// Compute the output path for a download given a resolved title.
    ///
    /// Playlists are handed a yt-dlp output template so each entry is named by
    /// yt-dlp; single videos get a sanitized `<title>.<ext>` file.
    pub fn output_path(&self, title: &str) -> PathBuf {
        let dir = self
            .output_dir
            .clone()
            .unwrap_or_else(utils::get_downloads_dir);
        if self.playlist {
            return dir.join("%(title)s.%(ext)s");
        }
        let ext = if self.format.as_deref() == Some("mp3") {
            "mp3"
        } else {
            "mp4"
        };
        dir.join(format!("{}.{}", sanitize_filename(title), ext))
    }

    /// A heads-up note when yt-dlp-only options (`-q`, `-f mp3`, `--subs`, clip
    /// section) are set but `url` is a direct media file — which the engine
    /// downloads as-is, ignoring those options. Returns `None` when the URL
    /// routes through yt-dlp (which honors them) or no such options are set.
    pub fn ignored_options_note(&self, url: &str) -> Option<String> {
        let has_ytdlp_only = self.quality.is_some()
            || self.format.as_deref() == Some("mp3")
            || self.subs
            || self.start_time.is_some()
            || self.end_time.is_some();
        if !has_ytdlp_only {
            return None;
        }

        let lower = url.to_lowercase();
        // The engine routes these through yt-dlp, which honors the options.
        if lower.contains("youtube.com/watch")
            || lower.contains("youtu.be/")
            || lower.contains(".m3u8")
            || lower.contains("/manifest")
            || lower.contains("playlist")
        {
            return None;
        }

        // A direct media-file URL is downloaded as-is by the engine.
        const MEDIA_EXTS: [&str; 9] = [
            ".mp4", ".mkv", ".webm", ".m4a", ".mp3", ".mov", ".avi", ".flv", ".ogg",
        ];
        let path = lower.split(['?', '#']).next().unwrap_or(&lower);
        if MEDIA_EXTS.iter().any(|e| path.ends_with(e)) {
            Some(
                "-q/-f/--subs/section flags apply to streaming-site downloads (yt-dlp); this \
                 looks like a direct file URL, which is downloaded as-is and ignores them."
                    .to_string(),
            )
        } else {
            None
        }
    }
}

/// Sanitize a video title into a safe single-path-component filename.
pub fn sanitize_filename(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "rustloader_download".to_string()
    } else {
        trimmed.chars().take(180).collect()
    }
}

/// Run a single headless download (or print the plan when `--dry-run`).
///
/// This drives A's existing engine: it builds the same [`HybridExtractor`] the
/// backend uses and calls [`DownloadEngine::download`].
pub async fn run(cli: &Cli) -> Result<()> {
    let url = cli
        .target_url()
        .context("no URL provided for CLI download")?
        .to_string();
    let options = cli.to_ytdlp_options();

    // `--dry-run` resolves the plan without any network I/O. It uses the same
    // argument builder the engine uses, proving the flags reach A's engine path.
    if cli.dry_run {
        let output_path = cli.output_path("video");
        let args = build_ytdlp_args(&options, &url, &output_path.to_string_lossy());
        println!("rustloader dry-run plan:");
        println!("  url:    {url}");
        println!("  output: {}", output_path.display());
        println!("  engine: DownloadEngine::download (yt-dlp path)");
        println!("  yt-dlp: yt-dlp {}", args.join(" "));
        return Ok(());
    }

    // Build the same hybrid extractor the backend uses: native extractors with
    // a yt-dlp fallback.
    let ytdlp = Arc::new(
        YtDlpExtractor::new()
            .context("failed to initialise yt-dlp extractor")?
            .with_cookies(cli.cookie_config()),
    );
    let extractor = HybridExtractor::new(Vec::new(), ytdlp);

    // Resolve a title for the output filename (best-effort; playlists skip this
    // and let yt-dlp name each entry via the output template).
    let title = if cli.playlist {
        "playlist".to_string()
    } else {
        match extractor.extract_info(&url).await {
            Ok(info) => info.title,
            Err(e) => {
                tracing::warn!("Could not extract video info ({e}); using a generic filename");
                "rustloader_download".to_string()
            }
        }
    };
    let output_path = cli.output_path(&title);

    // Configure the existing engine with the CLI-derived options and run it.
    let engine = DownloadEngine::default().with_ytdlp_options(options);

    // Heads-up rather than silent no-op: -q/-f/--subs/section flags only affect
    // the yt-dlp (streaming-site) path; a direct media-file URL is downloaded
    // as-is and ignores them.
    if let Some(note) = cli.ignored_options_note(&url) {
        eprintln!("⚠️  {note}");
    }

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<DownloadProgress>(100);
    tokio::spawn(async move {
        while let Some(p) = progress_rx.recv().await {
            println!(
                "Progress: {:.1}%  {:.2} MB/s  [{:?}]",
                p.percentage() * 100.0,
                p.speed / 1024.0 / 1024.0,
                p.status
            );
        }
    });

    println!("Downloading {url} -> {}", output_path.display());
    engine
        .download(&url, &output_path, progress_tx)
        .await
        .map_err(|e| {
            // Keep the raw error in the logs; show the user a friendly message.
            tracing::debug!("download failed (raw): {e:#}");
            anyhow::anyhow!("{}", crate::utils::make_error_user_friendly(&e.to_string()))
        })?;
    println!("Done.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_url_is_gui_mode() {
        let cli = Cli::try_parse_from(["rustloader"]).unwrap();
        assert!(!cli.is_cli_mode());
    }

    #[test]
    fn ignored_options_note_fires_for_direct_file() {
        // -q set + a direct media-file URL -> note.
        let cli = Cli::try_parse_from(["rustloader", "URL", "-q", "720"]).unwrap();
        assert!(cli
            .ignored_options_note("https://h/clip.mp4?token=1")
            .is_some());
        // Same options but a YouTube URL (yt-dlp honors them) -> no note.
        assert!(cli
            .ignored_options_note("https://www.youtube.com/watch?v=x")
            .is_none());
    }

    #[test]
    fn ignored_options_note_silent_without_options() {
        // No yt-dlp-only options set -> never a note, even for a direct file.
        let cli = Cli::try_parse_from(["rustloader", "URL"]).unwrap();
        assert!(cli.ignored_options_note("https://h/clip.mp4").is_none());
    }

    #[test]
    fn url_is_cli_mode() {
        let cli = Cli::try_parse_from(["rustloader", "https://youtu.be/abc"]).unwrap();
        assert_eq!(cli.target_url(), Some("https://youtu.be/abc"));
        assert!(cli.is_cli_mode());
    }

    #[test]
    fn flags_map_to_options() {
        let cli = Cli::try_parse_from([
            "rustloader",
            "URL",
            "-q",
            "720",
            "-f",
            "mp3",
            "-s",
            "00:00:10",
            "-e",
            "00:00:20",
            "--subs",
            "-p",
            "-o",
            "/tmp/out",
            "--bitrate",
            "128K",
        ])
        .unwrap();
        let opts = cli.to_ytdlp_options();
        assert_eq!(opts.quality, Some(720));
        assert!(opts.audio_only);
        assert_eq!(opts.audio_format.as_deref(), Some("mp3"));
        assert!(opts.subtitles);
        assert!(opts.playlist);
        assert_eq!(opts.start_time.as_deref(), Some("00:00:10"));
        assert_eq!(opts.end_time.as_deref(), Some("00:00:20"));
        assert_eq!(opts.audio_bitrate.as_deref(), Some("128K"));
    }

    #[test]
    fn rejects_invalid_quality() {
        assert!(Cli::try_parse_from(["rustloader", "URL", "-q", "4000"]).is_err());
    }

    #[test]
    fn rejects_invalid_format() {
        assert!(Cli::try_parse_from(["rustloader", "URL", "-f", "avi"]).is_err());
    }

    #[test]
    fn sanitize_filename_strips_separators() {
        assert_eq!(sanitize_filename("a/b:c?"), "a_b_c_");
        assert_eq!(sanitize_filename("   "), "rustloader_download");
    }

    #[test]
    fn output_path_uses_extension_for_format() {
        let cli = Cli::try_parse_from(["rustloader", "URL", "-o", "/tmp", "-f", "mp3"]).unwrap();
        assert_eq!(
            cli.output_path("My Song"),
            PathBuf::from("/tmp/My Song.mp3")
        );
        let cli = Cli::try_parse_from(["rustloader", "URL", "-o", "/tmp"]).unwrap();
        assert_eq!(cli.output_path("My Vid"), PathBuf::from("/tmp/My Vid.mp4"));
    }

    #[test]
    fn output_path_playlist_uses_template() {
        let cli = Cli::try_parse_from(["rustloader", "URL", "-o", "/tmp", "-p"]).unwrap();
        assert_eq!(
            cli.output_path("ignored"),
            PathBuf::from("/tmp/%(title)s.%(ext)s")
        );
    }
}
