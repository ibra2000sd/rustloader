//! yt-dlp cookie configuration shared by the extractor and the download engine.
//!
//! Some sites — notably YouTube, which may answer with
//! *"Sign in to confirm you're not a bot"* — require authenticated cookies for
//! yt-dlp to extract or download. This type carries the user's choice of cookie
//! source and renders it into the corresponding yt-dlp arguments, so the same
//! configuration can be applied to both the extraction call and the download
//! call from a single source of truth (CLI flags, GUI setting, or config).

use std::path::{Path, PathBuf};

/// Browsers yt-dlp can read cookies from, in preferred display order.
pub const SUPPORTED_BROWSERS: &[&str] = &[
    "firefox", "chrome", "safari", "edge", "brave", "chromium", "opera", "vivaldi",
];

/// Detect which supported browsers appear installed by probing their
/// profile/cookie locations under `home`. Returns names in
/// [`SUPPORTED_BROWSERS`] order. Pure (takes `home`) so it is unit-testable.
pub fn detect_browsers_in(home: &Path) -> Vec<String> {
    let lib = home.join("Library");
    // (browser, any-of these relative-to-~/Library paths exists)
    let candidates: &[(&str, &[&str])] = &[
        ("firefox", &["Application Support/Firefox/Profiles"]),
        ("chrome", &["Application Support/Google/Chrome"]),
        (
            "safari",
            &[
                "Containers/com.apple.Safari/Data/Library/Cookies",
                "Cookies/Cookies.binarycookies",
            ],
        ),
        ("edge", &["Application Support/Microsoft Edge"]),
        (
            "brave",
            &["Application Support/BraveSoftware/Brave-Browser"],
        ),
        ("chromium", &["Application Support/Chromium"]),
        ("opera", &["Application Support/com.operasoftware.Opera"]),
        ("vivaldi", &["Application Support/Vivaldi"]),
    ];
    candidates
        .iter()
        .filter(|(_, paths)| paths.iter().any(|p| lib.join(p).exists()))
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Detect installed browsers under the current user's home directory.
pub fn detect_browsers() -> Vec<String> {
    match dirs::home_dir() {
        Some(home) => detect_browsers_in(&home),
        None => Vec::new(),
    }
}

/// How to supply cookies to yt-dlp.
///
/// Both fields default to `None` (no cookies — the historical behaviour). When
/// set, they map onto yt-dlp's `--cookies-from-browser <browser>` and
/// `--cookies <file>` options respectively.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CookieConfig {
    /// `--cookies-from-browser <browser>` — e.g. `chrome`, `firefox`, `safari`,
    /// `edge`, `brave`. yt-dlp reads cookies directly from that browser's
    /// profile.
    pub from_browser: Option<String>,
    /// `--cookies <file>` — path to a Netscape-format `cookies.txt` file.
    pub file: Option<PathBuf>,
}

impl CookieConfig {
    /// Build from optional user-supplied values, treating empty/whitespace-only
    /// strings as "unset" so an empty GUI field or `--cookies-from-browser ""`
    /// doesn't emit a broken flag.
    pub fn new(from_browser: Option<String>, file: Option<PathBuf>) -> Self {
        let from_browser = from_browser
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let file = file.filter(|p| !p.as_os_str().is_empty());
        Self { from_browser, file }
    }

    /// True when at least one cookie source is configured.
    pub fn is_set(&self) -> bool {
        self.from_browser.is_some() || self.file.is_some()
    }

    /// Append the matching yt-dlp arguments (if any) to `args`. A default
    /// (empty) config appends nothing, leaving the command unchanged.
    pub fn append_args(&self, args: &mut Vec<String>) {
        if let Some(browser) = &self.from_browser {
            args.push("--cookies-from-browser".to_string());
            args.push(browser.clone());
        }
        if let Some(file) = &self.file {
            args.push("--cookies".to_string());
            args.push(file.to_string_lossy().to_string());
        }
    }

    /// The yt-dlp arguments for this config as a standalone vector.
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        self.append_args(&mut args);
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unset_and_emits_nothing() {
        let c = CookieConfig::default();
        assert!(!c.is_set());
        assert!(c.to_args().is_empty());
    }

    #[test]
    fn empty_strings_are_treated_as_unset() {
        let c = CookieConfig::new(Some("   ".to_string()), Some(PathBuf::from("")));
        assert!(!c.is_set());
        assert!(c.to_args().is_empty());
    }

    #[test]
    fn from_browser_emits_flag() {
        let c = CookieConfig::new(Some("chrome".to_string()), None);
        assert_eq!(
            c.to_args(),
            vec!["--cookies-from-browser".to_string(), "chrome".to_string()]
        );
    }

    #[test]
    fn file_emits_flag() {
        let c = CookieConfig::new(None, Some(PathBuf::from("/tmp/cookies.txt")));
        assert_eq!(
            c.to_args(),
            vec!["--cookies".to_string(), "/tmp/cookies.txt".to_string()]
        );
    }

    #[test]
    fn detect_browsers_finds_only_present_profiles() {
        let tmp = std::env::temp_dir().join(format!("rl-browsers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        // Create firefox + chrome profile dirs only.
        std::fs::create_dir_all(tmp.join("Library/Application Support/Firefox/Profiles")).unwrap();
        std::fs::create_dir_all(tmp.join("Library/Application Support/Google/Chrome")).unwrap();

        let found = detect_browsers_in(&tmp);
        assert!(found.contains(&"firefox".to_string()));
        assert!(found.contains(&"chrome".to_string()));
        assert!(!found.contains(&"brave".to_string()));
        assert!(!found.contains(&"edge".to_string()));
        // Order follows SUPPORTED_BROWSERS (firefox before chrome).
        let fi = found.iter().position(|b| b == "firefox").unwrap();
        let ci = found.iter().position(|b| b == "chrome").unwrap();
        assert!(fi < ci);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detect_browsers_empty_when_nothing_installed() {
        let tmp = std::env::temp_dir().join(format!("rl-nobrowsers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(detect_browsers_in(&tmp).is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn both_emit_both_flags() {
        let c = CookieConfig::new(
            Some("firefox".to_string()),
            Some(PathBuf::from("/tmp/c.txt")),
        );
        assert_eq!(
            c.to_args(),
            vec![
                "--cookies-from-browser".to_string(),
                "firefox".to_string(),
                "--cookies".to_string(),
                "/tmp/c.txt".to_string(),
            ]
        );
    }
}
