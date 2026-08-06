//! Error handling for Rustloader

use thiserror::Error;

/// Main error type for Rustloader
#[derive(Debug, Error)]
pub enum RustloaderError {
    #[error("yt-dlp not found. Please install yt-dlp")]
    YtDlpNotFound,

    #[error("Failed to extract video info: {0}")]
    ExtractionError(String),

    #[error("Download failed: {0}")]
    DownloadError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Map a raw error string (from yt-dlp / reqwest / the engine) to a concise,
/// user-friendly message. Shared by the GUI and the CLI so both surface the
/// same human-readable text instead of raw internals.
pub fn make_error_user_friendly(error: &str) -> String {
    let error_lower = error.to_lowercase();

    if error_lower.contains("truncated") || error_lower.contains("incomplete") {
        "Please enter a complete and valid URL".to_string()
    } else if error_lower.contains("invalid url") || error_lower.contains("malformed") {
        "This doesn't appear to be a valid video URL".to_string()
    } else if error_lower.contains("network")
        || error_lower.contains("connection")
        || error_lower.contains("timeout")
        || error_lower.contains("timed out")
        || error_lower.contains("dns")
        || error_lower.contains("resolve")
    {
        "Unable to connect. Please check your internet connection".to_string()
    } else if error_lower.contains("format") && error_lower.contains("not available") {
        // yt-dlp's "Requested format is not available": on YouTube this is
        // usually every real format having been skipped because the
        // n-challenge could not be solved (no JS runtime available to
        // yt-dlp), not a wrong quality choice. Checked before the generic
        // not-available arm so it doesn't read as "video removed".
        "No downloadable formats were found — the video may need a JavaScript runtime \
         (bundled with the app) or a different quality; try again or disable cookies \
         for this site"
            .to_string()
    } else if error_lower.contains("unavailable")
        || error_lower.contains("not available")
        || error_lower.contains("not found")
        || error_lower.contains("removed")
    {
        "This video is not available or has been removed".to_string()
    } else if error_lower.contains("age") && error_lower.contains("restricted") {
        // Before the private/restricted arm below: anything containing
        // "restricted" matched that first, so yt-dlp's age-gate errors
        // ("age-restricted", "confirm your age") were reported as a private
        // video — the wrong remedy, since an age gate can work with cookies
        // configured and a private video cannot.
        "This video is age-restricted and cannot be downloaded".to_string()
    } else if error_lower.contains("private") || error_lower.contains("restricted") {
        "This video is private or restricted".to_string()
    } else if error_lower.contains("geo") || error_lower.contains("region") {
        "This video is not available in your region".to_string()
    } else if error_lower.contains("copyright") {
        "This video cannot be downloaded due to copyright restrictions".to_string()
    } else if error_lower.contains("403") || error_lower.contains("forbidden") {
        "Download was blocked (HTTP 403). Try updating yt-dlp and installing a JS runtime (deno)"
            .to_string()
    } else {
        "Unable to process this URL. Please try a different video".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_maps_known_errors() {
        assert!(make_error_user_friendly("dns error: failed to resolve")
            .contains("check your internet"));
        assert!(make_error_user_friendly("HTTP Error 403: Forbidden").contains("403"));
        assert!(make_error_user_friendly("video is private").contains("private"));
        assert_eq!(
            make_error_user_friendly("some totally unexpected thing"),
            "Unable to process this URL. Please try a different video"
        );
    }

    #[test]
    fn friendly_maps_format_not_available_to_formats_message() {
        // The B-DL-009 shape: yt-dlp's real stderr when the n-challenge fails
        // and every real format is skipped (GUI sees it wrapped by
        // RustloaderError::ExtractionError).
        let raw = "Failed to extract video info: ERROR: [youtube] hJsRd6dRyr0: \
                   Requested format is not available. Use --list-formats for a \
                   list of available formats";
        let msg = make_error_user_friendly(raw);
        assert!(
            msg.contains("No downloadable formats"),
            "format errors must not flatten to the generic message: {msg}"
        );
    }

    #[test]
    fn friendly_maps_timed_out_to_network_message() {
        // The bounded-run error says "timed out", not "timeout"
        // (src/extractor/ytdlp.rs::run_bounded).
        assert!(make_error_user_friendly(
            "yt-dlp extraction timed out after 60s (subprocess killed)"
        )
        .contains("check your internet"));
    }

    /// The age arm sat AFTER `private || restricted`, so anything containing
    /// "restricted" — which every age-gate message does — was reported as a
    /// private video. Wrong remedy: an age gate can work once cookies are
    /// configured, a private video cannot.
    #[test]
    fn friendly_reports_age_gates_as_age_gates_not_as_private() {
        for raw in [
            "ERROR: [youtube] abc: Sign in to confirm your age. This video may be age-restricted",
            "This video is age restricted",
        ] {
            let msg = make_error_user_friendly(raw);
            assert!(
                msg.contains("age-restricted"),
                "expected an age-gate message for {raw:?}, got: {msg}"
            );
        }
    }

    #[test]
    fn friendly_still_reports_plain_private_videos_as_private() {
        assert_eq!(
            make_error_user_friendly("ERROR: [youtube] abc: Private video"),
            "This video is private or restricted"
        );
    }

    #[test]
    fn friendly_maps_plain_not_available_to_unavailable_message() {
        // Without the word "format", "not available" reads as the video being
        // gone — same bucket as "unavailable".
        assert_eq!(
            make_error_user_friendly("this video is not available"),
            "This video is not available or has been removed"
        );
    }
}
