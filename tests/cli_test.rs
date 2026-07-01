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
        "--experimental-aria2c",
    ] {
        assert!(
            stdout.contains(flag),
            "help should mention {flag}:\n{stdout}"
        );
    }
}

#[test]
fn help_labels_aria2c_flag_experimental() {
    // The flag must read as a clearly-labelled experimental opt-in with its
    // progress caveat, not a plain performance toggle.
    let out = std::process::Command::new(BIN)
        .arg("--help")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("EXPERIMENTAL"),
        "help should label --experimental-aria2c as experimental:\n{stdout}"
    );
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

#[test]
fn dry_run_omits_downloader_flag_by_default() {
    // F-DL-001b: absent --experimental-aria2c, the resolved args must be
    // byte-identical to before that flag existed -- no --downloader at all.
    let out = std::process::Command::new(BIN)
        .args(["https://example.com/video", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("--downloader"),
        "must not add --downloader without --experimental-aria2c:\n{stdout}"
    );
}

#[test]
fn dry_run_experimental_aria2c_adds_downloader_flag_when_aria2c_present() {
    // Environment-dependent (aria2c may not be installed on this machine or
    // CI runner) -- mirrors the existing find_aria2c/find_ytdlp smoke-test
    // idiom rather than requiring aria2c in CI.
    let aria2c_installed = std::process::Command::new("aria2c")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let out = std::process::Command::new(BIN)
        .args([
            "https://example.com/video",
            "--experimental-aria2c",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    if aria2c_installed {
        assert!(
            stdout.contains("--downloader aria2c"),
            "expected --downloader aria2c when aria2c is installed and the flag is set:\n{stdout}"
        );
    } else {
        // No aria2c reachable -- the flag must stay a safe no-op, never
        // fabricating a downloader that isn't actually there.
        assert!(
            !stdout.contains("--downloader"),
            "must not claim --downloader when aria2c isn't actually present:\n{stdout}"
        );
    }
}
