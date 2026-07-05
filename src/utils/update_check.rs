//! Launch-time update check against GitHub Releases (opt-out, notify-only).
//!
//! One GET to `/repos/ibra2000sd/rustloader/releases/latest` per launch, run
//! asynchronously so startup is never delayed. That endpoint already excludes
//! drafts and pre-releases, so rc tags never notify stable users. The banner
//! this feeds only *links* to the release page — the app never downloads or
//! replaces its own binary (in-app updating is a later Sparkle/axoupdater
//! step).
//!
//! Every failure mode — opted out, offline, timeout, 403/429 rate limit,
//! unexpected body, `304 Not Modified` — is a silent no-op logged at debug.
//! The check must never surface an error or nag.

use serde::Deserialize;
use std::time::Duration;

/// The releases/latest endpoint for this repo. Tests point `check_if_enabled`
/// at a local listener instead.
pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/ibra2000sd/rustloader/releases/latest";

/// Total request timeout. Unlike the download client's connect-only timeout
/// (B-DL-004), a whole-request bound is right here: the response is a small
/// JSON document, never a media transfer.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// A newer published release the UI may offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// Normalized version (tag with any leading `v`/`V` stripped), e.g. "1.2.0".
    pub version: String,
    /// The release page URL — the banner's Download opens this in the
    /// browser; nothing is fetched or installed by the app itself.
    pub html_url: String,
    /// Direct asset URLs (`assets[].browser_download_url`). Unused by the
    /// banner today; carried so a later platform-aware download step has them.
    pub asset_urls: Vec<String>,
}

/// Outcome of a check that got a usable `200` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    /// `Some` iff the latest release is strictly newer (semver) than the
    /// running version.
    pub update: Option<UpdateInfo>,
    /// The response's `ETag`, for `If-None-Match` on a later launch — a `304`
    /// answer costs nothing against GitHub's rate limit.
    pub etag: Option<String>,
}

/// The subset of the GitHub release object this feature reads.
#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    browser_download_url: String,
}

/// Run the update check, unless the user opted out — in which case return
/// immediately without building a client or touching the network.
pub async fn check_if_enabled(
    enabled: bool,
    api_url: &str,
    current_version: &str,
    prior_etag: Option<String>,
) -> Option<CheckOutcome> {
    if !enabled {
        return None;
    }
    match check(api_url, current_version, prior_etag).await {
        Ok(outcome) => Some(outcome),
        Err(e) => {
            tracing::debug!("update check skipped: {e:#}");
            None
        }
    }
}

async fn check(
    api_url: &str,
    current_version: &str,
    prior_etag: Option<String>,
) -> anyhow::Result<CheckOutcome> {
    let client = reqwest::Client::builder()
        // GitHub's API rejects requests without a User-Agent.
        .user_agent(concat!("rustloader/", env!("CARGO_PKG_VERSION")))
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    let mut request = client
        .get(api_url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(etag) = prior_etag {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }

    let response = request.send().await?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_MODIFIED {
        anyhow::bail!("304 Not Modified — nothing published since the last check");
    }
    if !status.is_success() {
        // 403/429 are GitHub's rate-limit answers; respecting the limit means
        // giving up quietly until the next launch, never retrying.
        let reset = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");
        anyhow::bail!("HTTP {status} (x-ratelimit-reset: {reset})");
    }

    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let body = response.text().await?;
    Ok(CheckOutcome {
        update: evaluate(&body, current_version)?,
        etag,
    })
}

/// Parse a releases/latest body and decide whether it names a newer release.
/// Pure (no I/O) so the decision is unit-testable against real API fixtures.
fn evaluate(body: &str, current_version: &str) -> anyhow::Result<Option<UpdateInfo>> {
    let release: LatestRelease = serde_json::from_str(body)?;
    if !is_newer(&release.tag_name, current_version) {
        return Ok(None);
    }
    Ok(Some(UpdateInfo {
        version: strip_v(&release.tag_name).to_string(),
        html_url: release.html_url,
        asset_urls: release
            .assets
            .into_iter()
            .map(|a| a.browser_download_url)
            .collect(),
    }))
}

/// True iff `latest_tag` (with an optional leading `v`/`V`) is strictly newer
/// than `current_version` under semver ordering — so "0.10.0" beats "0.9.0"
/// and "1.0.0-rc.1" does NOT beat "1.0.0". Anything unparsable can't be
/// proven newer and yields false (silent no-op, never a wrong banner).
pub fn is_newer(latest_tag: &str, current_version: &str) -> bool {
    match (
        semver::Version::parse(strip_v(latest_tag)),
        semver::Version::parse(current_version.trim()),
    ) {
        (Ok(latest), Ok(current)) => latest > current,
        _ => false,
    }
}

fn strip_v(tag: &str) -> &str {
    let tag = tag.trim();
    tag.strip_prefix(['v', 'V']).unwrap_or(tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{ErrorKind, Read, Write};
    use std::net::TcpListener;

    #[test]
    fn semver_compare_orders_correctly() {
        // Newer
        assert!(is_newer("0.9.1", "0.9.0"));
        assert!(is_newer("v1.0.0", "0.9.0"));
        // Numeric, not lexicographic: a string compare would call this older.
        assert!(is_newer("0.10.0", "0.9.0"));
        // Equal (with and without the v prefix)
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("v0.9.0", "0.9.0"));
        assert!(!is_newer("V0.9.0", "0.9.0"));
        // Older
        assert!(!is_newer("0.8.9", "0.9.0"));
        // Pre-release ordering: rc precedes its release …
        assert!(!is_newer("1.0.0-rc.1", "1.0.0"));
        // … and the release beats a running rc build.
        assert!(is_newer("1.0.0", "1.0.0-rc.1"));
        assert!(is_newer("1.0.0-rc.2", "1.0.0-rc.1"));
        // Garbage on either side can't be proven newer.
        assert!(!is_newer("not-a-version", "0.9.0"));
        assert!(!is_newer("v1.0.0", "garbage"));
        assert!(!is_newer("", ""));
    }

    /// Shape captured from the live endpoint on 2026-07-05 (fields trimmed).
    fn release_body(tag: &str) -> String {
        format!(
            r#"{{
              "tag_name": "{tag}",
              "html_url": "https://github.com/ibra2000sd/rustloader/releases/tag/{tag}",
              "draft": false,
              "prerelease": false,
              "assets": [
                {{"name": "rustloader-macos-arm64.tar.gz",
                  "browser_download_url": "https://github.com/ibra2000sd/rustloader/releases/download/{tag}/rustloader-macos-arm64.tar.gz"}}
              ]
            }}"#
        )
    }

    #[test]
    fn evaluate_flags_newer_release_with_link_and_assets() {
        let update = evaluate(&release_body("v0.10.0"), "0.9.0")
            .expect("parse")
            .expect("newer release must yield an update");
        assert_eq!(update.version, "0.10.0");
        assert_eq!(
            update.html_url,
            "https://github.com/ibra2000sd/rustloader/releases/tag/v0.10.0"
        );
        assert_eq!(update.asset_urls.len(), 1);
        assert!(update.asset_urls[0].ends_with("rustloader-macos-arm64.tar.gz"));
    }

    #[test]
    fn evaluate_is_quiet_when_up_to_date_or_behind() {
        assert_eq!(evaluate(&release_body("v0.9.0"), "0.9.0").unwrap(), None);
        assert_eq!(evaluate(&release_body("v0.8.0"), "0.9.0").unwrap(), None);
    }

    #[test]
    fn evaluate_rejects_non_json_bodies() {
        assert!(evaluate("<html>rate limited</html>", "0.9.0").is_err());
    }

    /// Serve exactly one canned HTTP/1.1 response on a local port, returning
    /// the raw request text so tests can assert on the headers we sent.
    fn one_shot_server(response: String) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}/", listener.local_addr().expect("addr"));
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                    Err(_) => break,
                }
            }
            stream.write_all(response.as_bytes()).expect("write");
            String::from_utf8_lossy(&request).into_owned()
        });
        (url, handle)
    }

    #[tokio::test]
    async fn opted_out_makes_no_request_at_all() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let url = format!("http://{}/", listener.local_addr().expect("addr"));

        let outcome = check_if_enabled(false, &url, "0.9.0", None).await;
        assert!(outcome.is_none());

        // Had a request been made, a connection would be queued on the
        // listener by now (everything above is sequential); WouldBlock
        // proves nothing ever connected.
        match listener.accept() {
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            other => panic!("expected no connection, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn newer_release_is_detected_end_to_end_and_headers_are_sent() {
        // Simulates "a newer release exists" against a real local HTTP
        // exchange — no stubbing inside the module under test.
        let body = release_body("v9.9.9");
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\netag: W/\"abc123\"\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let (url, server) = one_shot_server(response);

        let outcome = check_if_enabled(true, &url, "0.9.0", Some("W/\"old\"".to_string()))
            .await
            .expect("200 with newer tag must yield an outcome");
        let update = outcome.update.expect("v9.9.9 > 0.9.0");
        assert_eq!(update.version, "9.9.9");
        assert!(update.html_url.ends_with("/tag/v9.9.9"));
        assert_eq!(outcome.etag.as_deref(), Some("W/\"abc123\""));

        let request = server.join().expect("server thread").to_lowercase();
        assert!(
            request.contains(concat!(
                "user-agent: rustloader/",
                env!("CARGO_PKG_VERSION")
            )),
            "User-Agent header missing in: {request}"
        );
        assert!(
            request.contains("if-none-match: w/\"old\""),
            "If-None-Match header missing in: {request}"
        );
    }

    #[tokio::test]
    async fn not_modified_and_rate_limit_and_garbage_are_silent_noops() {
        for response in [
            "HTTP/1.1 304 Not Modified\r\ncontent-length: 0\r\n\r\n",
            "HTTP/1.1 403 Forbidden\r\nx-ratelimit-remaining: 0\r\nx-ratelimit-reset: 1783280232\r\ncontent-length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\ncontent-length: 9\r\n\r\nnot json!",
        ] {
            let (url, server) = one_shot_server(response.to_string());
            let outcome = check_if_enabled(true, &url, "0.9.0", None).await;
            assert!(outcome.is_none(), "expected silent no-op for {response:?}");
            server.join().expect("server thread");
        }
    }

    #[tokio::test]
    async fn offline_connection_refused_is_a_silent_noop() {
        // Bind then drop to get a port with nothing listening.
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").expect("bind");
            l.local_addr().expect("addr").port()
        };
        let url = format!("http://127.0.0.1:{port}/");
        assert!(check_if_enabled(true, &url, "0.9.0", None).await.is_none());
    }
}
