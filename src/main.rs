//! Rustloader - High-Performance Video Downloader
//!
//! A cross-platform video downloader that combines yt-dlp extraction capabilities
//! with a fast Rust-based download engine and a simple, practical GUI.

use anyhow::Result;
use clap::Parser;
use iced::Application;
use rustloader::cli::Cli;
use rustloader::gui;
use std::process::Command;

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
        if let Ok(output) = Command::new(path).arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("✓ yt-dlp found at: {} (version {})", path, version);
                ytdlp_version = Some(version);
                break;
            }
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
