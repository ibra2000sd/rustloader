//! Rustloader - High-Performance Video Downloader
//!
//! A cross-platform video downloader that combines yt-dlp extraction capabilities
//! with a fast Rust-based download engine and a simple, practical GUI.

use anyhow::Result;
use clap::Parser;
use iced::Application;
use rustloader::cli::Cli;
use rustloader::gui;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    // Must run before anything reads PATH or spawns a subprocess (the yt-dlp
    // probe and depcheck below, the download engine later), and before any
    // thread exists (`env::set_var` is only sound single-threaded).
    setup_bundled_tools_path();

    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Check for yt-dlp + emit non-blocking dependency health warnings.
    let startup_warnings = check_ytdlp_installed();

    if cli.is_cli_mode() {
        // Run a headless download through the existing engine inside a
        // temporary Tokio runtime, then exit (no GUI).
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async move { rustloader::cli::run(&cli).await })?;
        return Ok(());
    }

    // Start the GUI application (synchronous entrypoint). Startup warnings are
    // passed in as flags so the GUI can surface them as a banner.
    gui::RustloaderApp::run(iced::Settings {
        flags: startup_warnings,
        window: iced::window::Settings {
            size: iced::Size::new(900.0, 600.0),
            min_size: Some(iced::Size::new(800.0, 500.0)),
            // Native decorations stay ON: the merged titlebar below keeps the
            // real macOS traffic lights and native dragging — no custom-drawn
            // window buttons.
            decorations: true,
            // NOT window transparency: the app's own dark surface must fill
            // the titlebar area, not the desktop behind the window.
            transparent: false,
            icon: gui::icon::load_icon(), // Load application icon
            // macOS merged titlebar: hide the title text, make the titlebar
            // transparent, and extend the content to the top edge
            // (`titlebarAppearsTransparent` + `titleVisibility = .hidden` +
            // `.fullSizeContentView` via winit's `WindowBuilderExtMacOS`).
            // The traffic lights float over the sidebar, which reserves
            // top-left clearance for them (see `gui/app.rs`).
            #[cfg(target_os = "macos")]
            platform_specific: iced::window::settings::PlatformSpecific {
                title_hidden: true,
                titlebar_transparent: true,
                fullsize_content_view: true,
            },
            ..Default::default()
        },
        // Design-system typefaces (design-system/tokens/typography.css):
        // Geist for UI text, Geist Mono for numerals/data.
        fonts: gui::theme::font_bytes(),
        default_font: gui::theme::FONT_UI,
        antialiasing: true,
        ..Default::default()
    })?;

    Ok(())
}

/// Prepend the .app bundle's `Contents/Resources/bin` (yt-dlp, ffmpeg,
/// ffprobe, deno) to `PATH` when running from the macOS bundle.
///
/// The extractor resolves the bundled yt-dlp by path
/// (`platform::ytdlp_path`), but the download engine spawns `yt-dlp` by name,
/// yt-dlp finds `ffmpeg` and a JS runtime (deno, for YouTube's n-challenge)
/// on PATH, and `depcheck::has_js_runtime()` checks PATH too. Finder launches
/// apps with a minimal PATH — and possibly none at all — so a bare launch
/// falls back to the standard system directories: yt-dlp shells out to system
/// tools (e.g. `/usr/bin/security` for Chrome cookie decryption), so those
/// must stay reachable.
///
/// Outside a bundle (dev build, release archive, CLI) `bundled_bin_dir()` is
/// `None` and this is a no-op.
fn setup_bundled_tools_path() {
    let Some(bin_dir) = rustloader::utils::platform::bundled_bin_dir() else {
        return;
    };
    let base = std::env::var_os("PATH")
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "/usr/bin:/bin:/usr/sbin:/sbin".into());
    let paths = std::iter::once(bin_dir).chain(std::env::split_paths(&base));
    match std::env::join_paths(paths) {
        Ok(joined) => std::env::set_var("PATH", joined),
        Err(e) => eprintln!("WARNING: cannot put the bundled tools on PATH: {e}"),
    }
}

/// Upper bound on a single `--version` probe. Generous enough for a cold
/// binary (macOS scans a freshly installed one on first execution), short
/// enough that a wedged candidate can't hold the window shut for long.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Run `path --version` and return its trimmed stdout, giving up — and
/// killing the child — after `limit`.
///
/// I-1 requires every spawned binary to be awaited under an upper bound with a
/// guaranteed kill. This runs before any Tokio runtime exists, so the bound is
/// a `try_wait` poll loop rather than a timeout future. Without it a yt-dlp
/// that never answers (cold-start scan on a slow volume, a dead network mount,
/// a broken Python shim) hangs `main` before the window is created, and the
/// app simply never opens.
fn probe_version_bounded(path: &str, limit: Duration) -> Option<String> {
    let mut child = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                break;
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                eprintln!(
                    "WARNING: `{} --version` did not answer within {}s; giving up on it",
                    path,
                    limit.as_secs()
                );
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return None,
        }
    }

    let mut stdout = String::new();
    child.stdout.take()?.read_to_string(&mut stdout).ok()?;
    Some(stdout.trim().to_string())
}

/// Locate yt-dlp, then emit (non-blocking) dependency health warnings.
///
/// Returns the warnings so the GUI can also surface them as a banner.
fn check_ytdlp_installed() -> Vec<String> {
    use rustloader::utils::depcheck;

    // Try common macOS yt-dlp installation paths when launched from Finder/Dock
    // (where PATH may not include user-installed Python binaries)
    let possible_paths = [
        "yt-dlp", // Try PATH first
        "/usr/local/bin/yt-dlp",
        "/opt/homebrew/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.12/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.11/bin/yt-dlp",
        "/Library/Frameworks/Python.framework/Versions/3.10/bin/yt-dlp",
    ];

    let mut ytdlp_version: Option<String> = None;
    for path in &possible_paths {
        if let Some(version) = probe_version_bounded(path, PROBE_TIMEOUT) {
            println!("✓ yt-dlp found at: {} (version {})", path, version);
            ytdlp_version = Some(version);
            break;
        }
    }

    if ytdlp_version.is_none() {
        // yt-dlp not found - warn but don't exit, allow app to launch
        // User will see error when they try to add a URL
        eprintln!("WARNING: yt-dlp not found in common locations");
        eprintln!("The app will run, but video extraction will fail.");
        eprintln!("Please install yt-dlp:");
        eprintln!("  pip install yt-dlp");
        eprintln!("  or: brew install yt-dlp");
        eprintln!("  or visit: https://github.com/yt-dlp/yt-dlp");
    }

    // Non-blocking health check: stale yt-dlp and/or a missing JS runtime.
    let warnings = depcheck::health_warnings(
        ytdlp_version.as_deref(),
        chrono::Local::now().date_naive(),
        depcheck::has_js_runtime(),
    );
    for warning in &warnings {
        tracing::warn!("{}", warning);
        eprintln!("⚠️  {}", warning);
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write an executable stub script and return its path.
    #[cfg(unix)]
    fn stub(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write stub");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod stub");
        path
    }

    #[cfg(unix)]
    #[test]
    fn probe_returns_the_reported_version() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = stub(tmp.path(), "ok-yt-dlp.sh", "echo 2026.07.04");

        let version = probe_version_bounded(path.to_str().expect("utf-8"), PROBE_TIMEOUT);

        assert_eq!(version.as_deref(), Some("2026.07.04"));
    }

    #[cfg(unix)]
    #[test]
    fn probe_returns_none_for_a_failing_binary() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = stub(tmp.path(), "broken-yt-dlp.sh", "exit 1");

        assert_eq!(
            probe_version_bounded(path.to_str().expect("utf-8"), PROBE_TIMEOUT),
            None
        );
    }

    /// The regression: a probe that never answers used to hang `main` before
    /// the window was created, so the app never opened.
    #[cfg(unix)]
    #[test]
    fn probe_gives_up_on_a_binary_that_never_answers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = stub(tmp.path(), "hanging-yt-dlp.sh", "sleep 60");

        let limit = Duration::from_millis(300);
        let started = Instant::now();
        let version = probe_version_bounded(path.to_str().expect("utf-8"), limit);
        let elapsed = started.elapsed();

        assert_eq!(version, None, "a hung probe must not report a version");
        assert!(
            elapsed < Duration::from_secs(10),
            "probe must return near its {limit:?} bound, took {elapsed:?}"
        );
    }

    #[test]
    fn probe_returns_none_when_the_binary_does_not_exist() {
        assert_eq!(
            probe_version_bounded("definitely-not-a-real-yt-dlp-binary", PROBE_TIMEOUT),
            None
        );
    }
}
