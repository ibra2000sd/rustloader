//! Lightweight, non-blocking dependency health checks.
//!
//! These only emit WARN-level guidance — they never block startup or
//! auto-update. They catch the two failure modes that previously produced a
//! cryptic YouTube `403` with no hint: a yt-dlp that is suspiciously old, and a
//! missing JavaScript runtime (modern yt-dlp needs one to solve YouTube's JS
//! challenges).

use chrono::NaiveDate;

/// yt-dlp releases are versioned `YYYY.MM.DD`. Warn once a build is older than
/// this many days.
pub const YTDLP_STALE_DAYS: i64 = 60;

/// Parse a yt-dlp `YYYY.MM.DD` version string into a date.
pub fn parse_ytdlp_date(version: &str) -> Option<NaiveDate> {
    let mut parts = version.trim().split('.');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Age of a yt-dlp version in days relative to `today`, if it parses.
pub fn ytdlp_age_days(version: &str, today: NaiveDate) -> Option<i64> {
    parse_ytdlp_date(version).map(|released| (today - released).num_days())
}

/// True if a JavaScript runtime (`deno` or `node`) is on `PATH`.
pub fn has_js_runtime() -> bool {
    which::which("deno").is_ok() || which::which("node").is_ok()
}

/// Build the list of non-blocking health warnings.
///
/// Pure (all inputs injected) so it is unit-testable without a clock or PATH.
pub fn health_warnings(
    ytdlp_version: Option<&str>,
    today: NaiveDate,
    js_runtime_present: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if let Some(version) = ytdlp_version {
        if let Some(age) = ytdlp_age_days(version, today) {
            if age > YTDLP_STALE_DAYS {
                warnings.push(format!(
                    "yt-dlp {version} is {age} days old; YouTube extraction may fail. \
                     Update it (e.g. `pip install -U yt-dlp` or `brew upgrade yt-dlp`)."
                ));
            }
        }
    }

    if !js_runtime_present {
        warnings.push(
            "No JavaScript runtime (deno/node) found on PATH; modern yt-dlp needs one to \
             download from YouTube. Install Deno: `curl -fsSL https://deno.land/install.sh | sh`."
                .to_string(),
        );
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ytdlp_version_date() {
        assert_eq!(
            parse_ytdlp_date("2026.06.09"),
            NaiveDate::from_ymd_opt(2026, 6, 9)
        );
        assert_eq!(parse_ytdlp_date("not.a.date"), None);
        assert_eq!(parse_ytdlp_date("2026.06"), None);
    }

    #[test]
    fn stale_ytdlp_warns_past_threshold() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 29).unwrap();
        // 2025.11.12 is ~229 days old -> stale (js runtime present, so only this warning).
        let w = health_warnings(Some("2025.11.12"), today, true);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("yt-dlp 2025.11.12"));
    }

    #[test]
    fn fresh_ytdlp_does_not_warn() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 29).unwrap();
        // 2026.06.09 is 20 days old -> below the 60-day threshold.
        let w = health_warnings(Some("2026.06.09"), today, true);
        assert!(w.is_empty(), "fresh yt-dlp should not warn: {w:?}");
    }

    #[test]
    fn boundary_just_over_threshold_warns() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 29).unwrap();
        // 61 days old -> warns; 60 days -> does not.
        let stale = (today - chrono::Duration::days(YTDLP_STALE_DAYS + 1))
            .format("%Y.%m.%d")
            .to_string();
        let ok = (today - chrono::Duration::days(YTDLP_STALE_DAYS))
            .format("%Y.%m.%d")
            .to_string();
        assert_eq!(health_warnings(Some(&stale), today, true).len(), 1);
        assert!(health_warnings(Some(&ok), today, true).is_empty());
    }

    #[test]
    fn missing_js_runtime_warns() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 29).unwrap();
        let w = health_warnings(Some("2026.06.09"), today, false);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("JavaScript runtime"));
    }
}
