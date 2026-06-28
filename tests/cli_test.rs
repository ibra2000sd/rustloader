//! CLI surface tests.
//!
//! Parsing/mapping tests exercise the library `Cli` type; the binary-level
//! smoke tests run the built `rustloader` executable. None of these touch the
//! network — the download path is exercised via `--dry-run`, which resolves the
//! engine plan without performing any I/O.

use clap::Parser;
use rustloader::cli::Cli;

#[test]
fn parses_positional_url() {
    let cli = Cli::try_parse_from(["rustloader", "https://youtu.be/abc"]).unwrap();
    assert_eq!(cli.url.as_deref(), Some("https://youtu.be/abc"));
    assert!(cli.is_cli_mode());
}

#[test]
fn bare_invocation_is_gui_mode() {
    let cli = Cli::try_parse_from(["rustloader"]).unwrap();
    assert!(!cli.is_cli_mode());
}

#[test]
fn parses_every_flag() {
    let cli = Cli::try_parse_from([
        "rustloader",
        "URL",
        "-q",
        "1080",
        "-f",
        "mp4",
        "-s",
        "00:01:00",
        "-e",
        "00:02:00",
        "--subs",
        "-p",
        "-o",
        "/tmp/out",
        "--bitrate",
        "192K",
    ])
    .unwrap();
    assert_eq!(cli.quality.as_deref(), Some("1080"));
    assert_eq!(cli.format.as_deref(), Some("mp4"));
    assert_eq!(cli.start_time.as_deref(), Some("00:01:00"));
    assert_eq!(cli.end_time.as_deref(), Some("00:02:00"));
    assert!(cli.subs);
    assert!(cli.playlist);
    assert_eq!(cli.bitrate.as_deref(), Some("192K"));
}

#[test]
fn rejects_unsupported_quality() {
    assert!(Cli::try_parse_from(["rustloader", "URL", "-q", "8000"]).is_err());
}

#[test]
fn rejects_unsupported_format() {
    assert!(Cli::try_parse_from(["rustloader", "URL", "-f", "mkv"]).is_err());
}

// ---- Binary-level smoke tests (no network) ----

const BIN: &str = env!("CARGO_BIN_EXE_rustloader");

#[test]
fn help_succeeds_and_lists_surface() {
    let out = std::process::Command::new(BIN)
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for flag in [
        "--quality",
        "--format",
        "--subs",
        "--playlist",
        "--output-dir",
    ] {
        assert!(
            stdout.contains(flag),
            "help should mention {flag}:\n{stdout}"
        );
    }
}

#[test]
fn version_succeeds() {
    let out = std::process::Command::new(BIN)
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success(), "--version should exit 0");
}

#[test]
fn dry_run_flows_flags_into_engine_plan() {
    // Proves a CLI invocation reaches A's engine yt-dlp path with the flags
    // applied — without any network access.
    let out = std::process::Command::new(BIN)
        .args([
            "https://youtu.be/dQw4w9WgXcQ",
            "--subs",
            "-q",
            "720",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "dry-run should exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("DownloadEngine::download"),
        "plan must reference A's engine entry:\n{stdout}"
    );
    assert!(
        stdout.contains("--write-subs"),
        "subtitles flag must reach the yt-dlp args:\n{stdout}"
    );
    assert!(
        stdout.contains("height<=720"),
        "quality flag must reach the yt-dlp args:\n{stdout}"
    );
}
