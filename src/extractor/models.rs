//! Data structures for video information

use serde::{Deserialize, Serialize};

/// Video information structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    /// The canonical page URL. yt-dlp emits this as `webpage_url` (preferred,
    /// see `webpage_url` below) and/or a top-level `url`. Some sources emit
    /// BOTH — a `#[serde(alias = "webpage_url")]` on this field rejected that as
    /// a duplicate field, aborting the whole parse (losing the title too). So
    /// `url` now reads only the `url` key, and `normalize_url` prefers
    /// `webpage_url` when present.
    #[serde(default)]
    pub url: String,
    /// yt-dlp's `webpage_url`, captured separately to avoid a duplicate-field
    /// collision with `url`. Only used to normalize `url` after extraction;
    /// not persisted (so the event-log schema is unchanged).
    #[serde(default, skip_serializing)]
    pub webpage_url: Option<String>,
    #[serde(default)]
    pub direct_url: String, // Actual download URL (filled later)
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub filesize: Option<u64>,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub upload_date: Option<String>,
    #[serde(default)]
    pub formats: Vec<Format>,
    pub description: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub extractor: Option<String>,
}

/// Video format information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Format {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    #[serde(default)]
    pub filesize: Option<u64>,
    pub url: String,
    pub quality: Option<f32>,
    pub fps: Option<f32>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub format_note: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub tbr: Option<f32>, // Total bitrate
    pub vbr: Option<f32>, // Video bitrate
    pub abr: Option<f32>, // Audio bitrate
}

impl VideoInfo {
    /// Normalize `url` to yt-dlp's canonical `webpage_url` when present.
    ///
    /// Call this after deserializing yt-dlp's JSON: some sources emit only
    /// `webpage_url` (e.g. YouTube has no top-level `url`), others emit both.
    /// Capturing `webpage_url` separately avoids a duplicate-field parse error;
    /// this collapses it back into `url` so downstream code (organizer, GUI, DB)
    /// always sees the page URL.
    pub fn normalize_url(&mut self) {
        if let Some(w) = self.webpage_url.take() {
            if !w.is_empty() {
                self.url = w;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_with_both_url_and_webpage_url() {
        // Some direct-URL sources emit BOTH keys; the old alias rejected this
        // with "duplicate field `url`" and lost the title too.
        let json = r#"{"id":"x","title":"My Clip","url":"https://h/file.mp4",
            "webpage_url":"https://h/page"}"#;
        let mut v: VideoInfo = serde_json::from_str(json).expect("should not error on both keys");
        assert_eq!(v.title, "My Clip");
        v.normalize_url();
        assert_eq!(v.url, "https://h/page", "should prefer webpage_url");
    }

    #[test]
    fn parses_youtube_style_webpage_url_only() {
        let json = r#"{"id":"y","title":"Vid","webpage_url":"https://yt/watch?v=y"}"#;
        let mut v: VideoInfo = serde_json::from_str(json).expect("webpage_url only should parse");
        v.normalize_url();
        assert_eq!(v.url, "https://yt/watch?v=y");
    }

    #[test]
    fn parses_url_only_and_keeps_it() {
        let json = r#"{"id":"z","title":"Vid","url":"https://h/only.mp4"}"#;
        let mut v: VideoInfo = serde_json::from_str(json).expect("url only should parse");
        v.normalize_url();
        assert_eq!(v.url, "https://h/only.mp4");
    }

    #[test]
    fn webpage_url_is_not_serialized() {
        let v = VideoInfo {
            id: "a".into(),
            title: "t".into(),
            url: "https://h/p".into(),
            webpage_url: Some("https://h/p".into()),
            ..Default::default()
        };
        let s = serde_json::to_string(&v).unwrap();
        assert!(
            !s.contains("webpage_url"),
            "webpage_url must be skipped on serialize (event-log schema unchanged): {s}"
        );
    }
}
