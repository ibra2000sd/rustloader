//! Pure helpers for opt-in clipboard monitoring.
//!
//! The GUI polls the clipboard on a timer (only while the Settings toggle is
//! ON) and uses these helpers to decide whether the current clipboard text is
//! a *newly copied* http(s) URL worth offering to download. Everything here is
//! pure and synchronous so it can be unit-tested without a clipboard; the
//! actual read stays in `gui::clipboard::get_clipboard_content()`.
//!
//! Privacy contract: clipboard text is only compared/parsed in memory. It is
//! never persisted, logged, or transmitted; non-URL content is ignored
//! silently.

/// Return the trimmed clipboard text if (and only if) it looks like a single
/// http(s) URL. Anything else — other schemes, multi-line text, prose,
/// scheme-only strings — yields `None`.
pub fn detect_url(content: &str) -> Option<String> {
    let trimmed = content.trim();
    // A URL is a single token: internal whitespace or control chars mean this
    // is prose or a multi-line selection, not a copied link.
    if trimmed.is_empty() || trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }

    // http(s) only, scheme matched case-insensitively (byte-wise, so no risk
    // of slicing inside a multi-byte character).
    let bytes = trimmed.as_bytes();
    let scheme_len = if bytes.len() > 8 && bytes[..8].eq_ignore_ascii_case(b"https://") {
        8
    } else if bytes.len() > 7 && bytes[..7].eq_ignore_ascii_case(b"http://") {
        7
    } else {
        return None;
    };

    // The authority (up to the first `/`, `?`, or `#`) must contain a
    // non-empty host once optional userinfo and port are stripped.
    let rest = &trimmed[scheme_len..];
    let authority = rest.split(&['/', '?', '#'][..]).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default();
    if host.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

/// De-dup tracker for the clipboard monitor.
///
/// Remembers the last clipboard value it has seen so that:
/// - the same copied URL is surfaced once, not on every poll tick;
/// - content already on the clipboard when monitoring starts is treated as
///   seed state, not as a fresh copy (the first observation never prompts);
/// - the app's own paste action can mark its content as handled
///   ([`ClipboardWatch::mark_seen`]) so the monitor doesn't re-offer it.
#[derive(Debug, Default)]
pub struct ClipboardWatch {
    last_seen: Option<String>,
}

impl ClipboardWatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the current clipboard text to the watch. Returns `Some(url)` only
    /// when the content is a monitorable http(s) URL that differs from the
    /// previously seen clipboard value. The very first observation after
    /// (re)start only seeds the state and never returns a URL.
    pub fn observe(&mut self, content: &str) -> Option<String> {
        let is_first = self.last_seen.is_none();
        if self.last_seen.as_deref() == Some(content) {
            return None;
        }
        self.last_seen = Some(content.to_string());
        if is_first {
            return None;
        }
        detect_url(content)
    }

    /// Record `content` as already handled (e.g. the user pasted it manually)
    /// without offering it for download.
    pub fn mark_seen(&mut self, content: &str) {
        self.last_seen = Some(content.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plain_http_and_https_urls() {
        assert_eq!(
            detect_url("https://example.com/watch?v=abc"),
            Some("https://example.com/watch?v=abc".to_string())
        );
        assert_eq!(
            detect_url("http://example.com"),
            Some("http://example.com".to_string())
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            detect_url("  https://example.com/video \n"),
            Some("https://example.com/video".to_string())
        );
    }

    #[test]
    fn scheme_match_is_case_insensitive() {
        assert_eq!(
            detect_url("HTTPS://Example.com/A"),
            Some("HTTPS://Example.com/A".to_string())
        );
    }

    #[test]
    fn accepts_userinfo_and_port() {
        assert_eq!(
            detect_url("https://user@example.com:8443/x"),
            Some("https://user@example.com:8443/x".to_string())
        );
    }

    #[test]
    fn rejects_non_http_schemes() {
        assert_eq!(detect_url("ftp://example.com/file"), None);
        assert_eq!(detect_url("file:///etc/passwd"), None);
        assert_eq!(detect_url("javascript:alert(1)"), None);
    }

    #[test]
    fn rejects_non_url_text() {
        assert_eq!(detect_url(""), None);
        assert_eq!(detect_url("   "), None);
        assert_eq!(detect_url("hello world"), None);
        assert_eq!(detect_url("some password I copied"), None);
    }

    #[test]
    fn rejects_urls_embedded_in_prose_or_multiline_text() {
        assert_eq!(detect_url("see https://example.com for details"), None);
        assert_eq!(detect_url("https://a.com\nhttps://b.com"), None);
    }

    #[test]
    fn rejects_scheme_without_host() {
        assert_eq!(detect_url("https://"), None);
        assert_eq!(detect_url("http:///path-only"), None);
        assert_eq!(detect_url("https://@:80/x"), None);
    }

    #[test]
    fn first_observation_seeds_without_prompting() {
        let mut watch = ClipboardWatch::new();
        // A URL already on the clipboard when monitoring starts is not a new
        // copy — it must not prompt.
        assert_eq!(watch.observe("https://stale.example.com"), None);
        // …but a subsequent, different URL does.
        assert_eq!(
            watch.observe("https://fresh.example.com"),
            Some("https://fresh.example.com".to_string())
        );
    }

    #[test]
    fn same_content_does_not_reprompt() {
        let mut watch = ClipboardWatch::new();
        assert_eq!(watch.observe("seed"), None);
        assert_eq!(
            watch.observe("https://example.com/v"),
            Some("https://example.com/v".to_string())
        );
        // Every following tick sees the same clipboard value: no loop.
        assert_eq!(watch.observe("https://example.com/v"), None);
        assert_eq!(watch.observe("https://example.com/v"), None);
    }

    #[test]
    fn non_url_content_is_ignored_but_still_deduped() {
        let mut watch = ClipboardWatch::new();
        assert_eq!(watch.observe("seed"), None);
        assert_eq!(watch.observe("just some text"), None);
        // The non-URL updated last_seen, so a URL after it still prompts once.
        assert_eq!(
            watch.observe("https://example.com"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn mark_seen_suppresses_the_prompt_for_pasted_content() {
        let mut watch = ClipboardWatch::new();
        assert_eq!(watch.observe("seed"), None);
        // The user pasted this URL into the input themselves; the monitor
        // must not re-offer it on the next tick.
        watch.mark_seen("https://example.com/pasted");
        assert_eq!(watch.observe("https://example.com/pasted"), None);
        // A genuinely new copy afterwards still prompts.
        assert_eq!(
            watch.observe("https://example.com/new"),
            Some("https://example.com/new".to_string())
        );
    }
}
