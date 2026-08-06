//! Application configuration
#![allow(unused_imports)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Download location
    pub download_location: PathBuf,

    /// Number of segments per download
    pub segments: usize,

    /// Maximum concurrent downloads
    pub max_concurrent: usize,

    /// Preferred video quality
    pub quality: VideoQuality,

    /// Chunk size for streaming (bytes)
    pub chunk_size: usize,

    /// Retry attempts per segment
    pub retry_attempts: usize,

    /// Enable resume capability
    pub enable_resume: bool,

    /// Browser to read cookies from for authenticated sites (yt-dlp
    /// `--cookies-from-browser`), e.g. "chrome"/"firefox"/"safari". `None` =
    /// no browser cookies.
    #[serde(default)]
    pub cookies_from_browser: Option<String>,

    /// Path to a Netscape-format cookies.txt file (yt-dlp `--cookies`). `None` =
    /// no cookies file.
    #[serde(default)]
    pub cookies_file: Option<PathBuf>,

    /// Opt-in clipboard monitoring: when true, the GUI watches the clipboard
    /// for newly copied http(s) URLs and offers to download them (with
    /// confirmation). Privacy-sensitive, so it defaults to OFF.
    #[serde(default)]
    pub clipboard_monitoring: bool,

    /// Preferred output format. `Best` (the default) keeps whatever container
    /// the source's best streams produce — the historical behaviour, no
    /// conversion.
    #[serde(default)]
    pub output_format: OutputFormat,

    /// Once per launch, ask GitHub whether a newer release exists and show a
    /// dismissible banner if so (notify-only — nothing downloads or installs).
    /// The check is a single request to api.github.com, so GitHub sees the
    /// machine's IP address; opt-out in Settings. Defaults ON.
    #[serde(default = "default_check_updates_on_launch")]
    pub check_updates_on_launch: bool,

    /// Opt-in browser-integration bridge (F-EXT-001): when true, the GUI
    /// runs a loopback-only HTTP server the paired browser extension posts
    /// download requests to. Defaults OFF.
    #[serde(default)]
    pub browser_bridge: bool,

    /// Pairing token for the browser bridge, generated on first enable and
    /// persisted so the extension stays paired across restarts. Shown in
    /// Settings; never sent anywhere by the app.
    #[serde(default)]
    pub bridge_token: Option<String>,
}

fn default_check_updates_on_launch() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        // Use bundle-aware path resolution to ensure downloads go to ~/Downloads
        // even when the app is launched from Finder/Dock (where cwd is "/").
        // Never use relative paths like "./downloads" as they fail in non-Terminal launches.
        Self {
            download_location: crate::utils::get_downloads_dir(),
            segments: 16,
            max_concurrent: 5,
            quality: VideoQuality::Best,
            chunk_size: 8192, // 8KB
            retry_attempts: 3,
            enable_resume: true,
            cookies_from_browser: None,
            cookies_file: None,
            clipboard_monitoring: false,
            output_format: OutputFormat::Best,
            check_updates_on_launch: true,
            browser_bridge: false,
            bridge_token: None,
        }
    }
}

/// The user's output format choice (B-GUI-005 — the "Format" control used to
/// be a dead static "MP4" label).
///
/// Semantics per variant, applied on the engine's yt-dlp path:
/// - `Best`: no conversion at all — the file keeps whatever container the
///   source's best streams merge into (YouTube "Best" is typically AV1/Opus
///   in .webm). Byte-identical to the pre-selector behaviour.
/// - Video containers (`Mp4`/`Webm`/`Mkv`): yt-dlp `--remux-video <ext>` — a
///   lossless container change, NOT a re-encode. If ffmpeg can't fit the
///   source codecs into the target container (e.g. H.264 into WebM), the
///   engine keeps the original container instead of failing the download.
/// - Audio (`Mp3`/`M4a`/`Opus`/`Flac`/`Wav`): yt-dlp `-x --audio-format
///   <fmt>` — audio-only output via the engine's existing audio path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    #[default]
    Best,
    Mp4,
    Webm,
    Mkv,
    Mp3,
    M4a,
    Opus,
    Flac,
    Wav,
}

impl OutputFormat {
    /// Every variant, in the order the GUI dropdown shows them.
    pub const ALL: [OutputFormat; 9] = [
        OutputFormat::Best,
        OutputFormat::Mp4,
        OutputFormat::Webm,
        OutputFormat::Mkv,
        OutputFormat::Mp3,
        OutputFormat::M4a,
        OutputFormat::Opus,
        OutputFormat::Flac,
        OutputFormat::Wav,
    ];

    /// Human label shown in the dropdown.
    pub fn label(&self) -> &'static str {
        match self {
            OutputFormat::Best => "Original (Best)",
            OutputFormat::Mp4 => "MP4",
            OutputFormat::Webm => "WebM",
            OutputFormat::Mkv => "MKV",
            OutputFormat::Mp3 => "MP3 (audio)",
            OutputFormat::M4a => "M4A (audio)",
            OutputFormat::Opus => "Opus (audio)",
            OutputFormat::Flac => "FLAC (audio)",
            OutputFormat::Wav => "WAV (audio)",
        }
    }

    /// Parse a dropdown label back to the variant (`None` for unknown text).
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.iter().find(|f| f.label() == label).cloned()
    }

    /// Canonical persistence key (settings table value).
    pub fn as_key(&self) -> &'static str {
        match self {
            OutputFormat::Best => "best",
            OutputFormat::Mp4 => "mp4",
            OutputFormat::Webm => "webm",
            OutputFormat::Mkv => "mkv",
            OutputFormat::Mp3 => "mp3",
            OutputFormat::M4a => "m4a",
            OutputFormat::Opus => "opus",
            OutputFormat::Flac => "flac",
            OutputFormat::Wav => "wav",
        }
    }

    /// Parse a persisted key; anything unknown loads as `Best` so a
    /// downgraded/edited settings row can never break startup.
    pub fn from_key(key: &str) -> Self {
        Self::ALL
            .iter()
            .find(|f| f.as_key() == key)
            .cloned()
            .unwrap_or(OutputFormat::Best)
    }

    /// Target extension for yt-dlp `--remux-video`, when this is a video
    /// container choice.
    pub fn remux_ext(&self) -> Option<&'static str> {
        match self {
            OutputFormat::Mp4 => Some("mp4"),
            OutputFormat::Webm => Some("webm"),
            OutputFormat::Mkv => Some("mkv"),
            _ => None,
        }
    }

    /// Target format for yt-dlp `-x --audio-format`, when this is an audio
    /// choice.
    pub fn audio_ext(&self) -> Option<&'static str> {
        match self {
            OutputFormat::Mp3 => Some("mp3"),
            OutputFormat::M4a => Some("m4a"),
            OutputFormat::Opus => Some("opus"),
            OutputFormat::Flac => Some("flac"),
            OutputFormat::Wav => Some("wav"),
            _ => None,
        }
    }

    /// True for the no-conversion default.
    pub fn is_best(&self) -> bool {
        matches!(self, OutputFormat::Best)
    }
}

/// Video quality options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoQuality {
    Best,
    Worst,
    Specific(String), // Format ID
}

impl VideoQuality {
    /// Get string representation for display
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoQuality::Best => "Best Available",
            VideoQuality::Worst => "Worst Available",
            VideoQuality::Specific(_) => "Custom",
        }
    }

    /// The label as it appears in the quality dropdowns.
    ///
    /// Distinct from [`as_str`]: a `pick_list` shows a selection only when it
    /// equals one of its options, and the options are "480p"-style strings, so
    /// `as_str`'s "Custom" renders as a blank box. Both the main view and
    /// Settings use this, so the two can never disagree about what is
    /// selected.
    pub fn display_label(&self) -> String {
        match self {
            VideoQuality::Best => "Best Available".to_string(),
            VideoQuality::Worst => "Worst Available".to_string(),
            VideoQuality::Specific(height) => format!("{height}p"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppSettings::default();
        assert!(config.max_concurrent > 0);
        assert!(config.segments > 0);
        assert!(config.chunk_size > 0);
        assert!(config.retry_attempts > 0);
    }

    #[test]
    fn test_config_validation_like_defaults() {
        let mut config = AppSettings {
            max_concurrent: 0,
            segments: 0,
            ..Default::default()
        };

        // Enforce sane minimums
        if config.max_concurrent == 0 {
            config.max_concurrent = 1;
        }
        if config.segments == 0 {
            config.segments = 1;
        }

        assert_eq!(config.max_concurrent, 1);
        assert_eq!(config.segments, 1);
    }

    #[test]
    fn output_format_label_and_key_round_trip() {
        for f in OutputFormat::ALL {
            assert_eq!(OutputFormat::from_label(f.label()), Some(f.clone()));
            assert_eq!(OutputFormat::from_key(f.as_key()), f);
        }
        // Unknown persisted values must degrade to Best, not break startup.
        assert_eq!(
            OutputFormat::from_key("definitely-not-a-format"),
            OutputFormat::Best
        );
        assert_eq!(OutputFormat::from_label("nope"), None);
    }

    #[test]
    fn output_format_maps_to_exactly_one_ytdlp_action() {
        for f in OutputFormat::ALL {
            let actions = [
                f.is_best(),
                f.remux_ext().is_some(),
                f.audio_ext().is_some(),
            ];
            assert_eq!(
                actions.iter().filter(|a| **a).count(),
                1,
                "{f:?} must be exactly one of best/remux/audio"
            );
        }
    }
}

#[cfg(test)]
mod quality_label_tests {
    use super::VideoQuality;

    /// The Settings picker used to be hardcoded to "Best Available", and
    /// `as_str` returns "Custom" for a specific height — a value matching no
    /// dropdown option, which iced renders as an empty box. Both views now
    /// take their label from here.
    #[test]
    fn every_quality_matches_a_dropdown_option() {
        let options = ["Best Available", "Worst Available", "1080p", "720p", "480p"];

        for quality in [
            VideoQuality::Best,
            VideoQuality::Worst,
            VideoQuality::Specific("1080".to_string()),
            VideoQuality::Specific("720".to_string()),
            VideoQuality::Specific("480".to_string()),
        ] {
            let label = quality.display_label();
            assert!(
                options.contains(&label.as_str()),
                "{quality:?} renders as {label:?}, which matches no picker option"
            );
        }
    }

    #[test]
    fn a_specific_height_is_not_reported_as_custom() {
        assert_eq!(
            VideoQuality::Specific("480".to_string()).display_label(),
            "480p"
        );
        assert_ne!(
            VideoQuality::Specific("480".to_string()).display_label(),
            VideoQuality::Best.display_label(),
            "a chosen height must not be shown as Best Available"
        );
    }
}
