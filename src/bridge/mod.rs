//! Local loopback bridge for the browser extension (F-EXT-001, Phase 1).
//!
//! A tiny hand-rolled HTTP/1.1 server — the same idiom the test suites in
//! `downloader/segment.rs` and `downloader/engine.rs` already use — that the
//! Chrome extension talks to. Design: `docs/browser-integration-design.md`.
//!
//! Security posture (non-negotiable, see the design doc §6):
//! - binds `127.0.0.1` **only** ([`bind`] hard-codes the loopback address);
//! - every request must carry the pairing token except `GET /api/v1/ping`,
//!   which never reveals it (it only reports whether the caller's token
//!   matched, so the extension can distinguish "rustloader, paired" from
//!   "rustloader, bad token" from "some other app");
//! - token comparison is constant-time ([`constant_time_eq`]);
//! - the `Host` header must be a loopback name (DNS-rebinding defence);
//! - request size is capped (16 KiB headers, 1 MiB body);
//! - per-connection handling is time-bounded (the spirit of invariant I-1).
//!
//! Cookies arrive as JSON records and are written to a Netscape-format
//! `cookies.txt` under the app-support `bridge/` directory, then handed to
//! the rest of the app as a path for `CookieConfig { file }` — invariant I-7:
//! no `--cookies*` flag is assembled here or anywhere outside `CookieConfig`.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// Fixed discovery range: the extension probes these ports with
/// `GET /api/v1/ping` and uses the first that answers as rustloader.
/// Uncommon base on purpose (nothing popular squats 4615x), 5 ports so a
/// stale/foreign process on one of them doesn't brick the bridge.
/// Mirrored in `extension/chrome/bridge-client.js` — keep the two in sync.
pub const BRIDGE_PORTS: &[u16] = &[46150, 46151, 46152, 46153, 46154];

/// Bridge protocol version, reported by `/api/v1/ping`.
pub const API_VERSION: u32 = 1;

const MAX_HEAD_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;
const CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// A download request accepted by the bridge, handed to the GUI.
///
/// `cookies_file` is a Netscape-format temp file already written to disk
/// (mode 0600 on unix); the GUI passes it through `CookieConfig` and deletes
/// it once extraction finishes.
#[derive(Debug, Clone)]
pub struct BridgeRequest {
    pub url: String,
    pub cookies_file: Option<PathBuf>,
    /// Requested quality: "Best" / "Worst" / a height like "1080". `None` =
    /// use the app's configured default.
    pub quality: Option<String>,
    /// Requested output format, one of `OutputFormat::as_key` (e.g. "mp4",
    /// "mp3"). `None` = the app's configured default.
    pub output_format: Option<String>,
}

/// Events the bridge surfaces to the GUI subscription.
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// The server is listening on this loopback port.
    Started(u16),
    /// A paired extension posted a download request.
    Request(BridgeRequest),
    /// The server could not start (e.g. every port in the range is taken).
    Failed(String),
}

/// Generate a 128-bit random pairing token as 32 lowercase hex chars.
pub fn generate_token() -> String {
    let mut bytes = [0u8; 16];
    // getrandom only fails when the OS RNG is unavailable — treat that as
    // fatal for token generation rather than falling back to something weak.
    getrandom::getrandom(&mut bytes).expect("OS random number generator unavailable");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Constant-time string equality for token comparison.
///
/// The comparison XOR-folds over every byte of both inputs regardless of
/// where the first mismatch is; only the (public) lengths short-circuit.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Bind the bridge listener: loopback only, first free port in
/// [`BRIDGE_PORTS`]. Never binds a non-loopback address.
pub async fn bind() -> std::io::Result<TcpListener> {
    let mut last_err = None;
    for &port in BRIDGE_PORTS {
        match TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
            Ok(listener) => return Ok(listener),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::AddrInUse, "no bridge port available")
    }))
}

/// Accept loop. Runs until the listener (owned by the caller's future) is
/// dropped. Each connection is handled in its own time-bounded task.
pub async fn serve(
    listener: TcpListener,
    token: String,
    cookie_dir: PathBuf,
    tx: mpsc::Sender<BridgeRequest>,
) {
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(_) => 0,
    };
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                // Belt-and-braces: the bind is loopback-only, but refuse any
                // non-loopback peer outright anyway.
                if !peer.ip().is_loopback() {
                    continue;
                }
                let token = token.clone();
                let cookie_dir = cookie_dir.clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    let _ = tokio::time::timeout(
                        CONNECTION_TIMEOUT,
                        handle_connection(stream, port, token, cookie_dir, tx),
                    )
                    .await;
                });
            }
            Err(e) => {
                tracing::warn!("bridge: accept failed: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// One cookie record as sent by the extension (`chrome.cookies.getAll`).
#[derive(Debug, Clone, Deserialize)]
pub struct CookieRecord {
    pub name: String,
    pub value: String,
    pub domain: String,
    #[serde(default = "default_cookie_path")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default, alias = "httpOnly")]
    pub http_only: bool,
    /// Unix seconds; absent/0 = session cookie.
    #[serde(default)]
    pub expires: Option<f64>,
}

fn default_cookie_path() -> String {
    "/".to_string()
}

#[derive(Debug, Deserialize)]
struct DownloadBody {
    url: String,
    #[serde(default)]
    cookies: Option<Vec<CookieRecord>>,
    /// Accepted for forward-compatibility (referer/user-agent); not applied
    /// in Phase 1.
    #[serde(default)]
    #[allow(dead_code)]
    headers: Option<serde_json::Value>,
    #[serde(default)]
    quality: Option<String>,
    #[serde(default)]
    output_format: Option<String>,
}

struct Request {
    method: String,
    path: String,
    host: Option<String>,
    authorization: Option<String>,
    body: Vec<u8>,
}

async fn handle_connection(
    mut stream: TcpStream,
    port: u16,
    token: String,
    cookie_dir: PathBuf,
    tx: mpsc::Sender<BridgeRequest>,
) {
    let response = match read_request(&mut stream).await {
        Ok(req) => route(req, port, &token, &cookie_dir, &tx).await,
        Err(e) => e,
    };
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Read and parse one HTTP/1.1 request. Errors are returned as ready-to-send
/// HTTP responses.
async fn read_request(stream: &mut TcpStream) -> Result<Request, String> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];

    // Read until end of headers.
    let head_end = loop {
        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
            break pos;
        }
        if buf.len() > MAX_HEAD_BYTES {
            return Err(json_response(431, r#"{"error":"headers too large"}"#));
        }
        match stream.read(&mut chunk).await {
            Ok(0) => return Err(json_response(400, r#"{"error":"truncated request"}"#)),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => return Err(json_response(400, r#"{"error":"read error"}"#)),
        }
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_uppercase();
    let path = parts.next().unwrap_or_default().to_string();

    let mut host = None;
    let mut authorization = None;
    let mut content_length: usize = 0;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "host" => host = Some(value.to_string()),
            "authorization" => authorization = Some(value.to_string()),
            "content-length" => {
                content_length = value
                    .parse()
                    .map_err(|_| json_response(400, r#"{"error":"bad content-length"}"#))?;
            }
            _ => {}
        }
    }

    if content_length > MAX_BODY_BYTES {
        return Err(json_response(413, r#"{"error":"body too large"}"#));
    }

    // Read the body (whatever arrived past the headers, then the rest).
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        match stream.read(&mut chunk).await {
            Ok(0) => return Err(json_response(400, r#"{"error":"truncated body"}"#)),
            Ok(n) => body.extend_from_slice(&chunk[..n]),
            Err(_) => return Err(json_response(400, r#"{"error":"read error"}"#)),
        }
        if body.len() > MAX_BODY_BYTES {
            return Err(json_response(413, r#"{"error":"body too large"}"#));
        }
    }
    body.truncate(content_length);

    Ok(Request {
        method,
        path,
        host,
        authorization,
        body,
    })
}

/// True when the `Host` header names this loopback server (with or without
/// the port). Anything else — a real hostname resolving here via DNS
/// rebinding, another IP — is rejected.
fn host_is_loopback(host: &str, port: u16) -> bool {
    let name = host
        .strip_suffix(&format!(":{port}"))
        .unwrap_or(host)
        .to_ascii_lowercase();
    name == "127.0.0.1" || name == "localhost"
}

/// The bearer token from an `Authorization` header, if it is well-formed.
fn bearer_token(authorization: Option<&str>) -> Option<&str> {
    authorization?.strip_prefix("Bearer ").map(str::trim)
}

async fn route(
    req: Request,
    port: u16,
    token: &str,
    cookie_dir: &Path,
    tx: &mpsc::Sender<BridgeRequest>,
) -> String {
    match req.host.as_deref() {
        Some(h) if host_is_loopback(h, port) => {}
        _ => return json_response(403, r#"{"error":"forbidden host"}"#),
    }

    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/v1/ping") => {
            // Never reveal the token: only report whether the caller's
            // matched, so the extension can tell rustloader apart from a
            // stranger on the port AND validate its stored token.
            let paired = bearer_token(req.authorization.as_deref())
                .map(|t| constant_time_eq(t, token))
                .unwrap_or(false);
            json_response(
                200,
                &format!(
                    r#"{{"app":"rustloader","api":{API_VERSION},"version":"{}","paired":{paired}}}"#,
                    env!("CARGO_PKG_VERSION")
                ),
            )
        }
        ("POST", "/api/v1/download") => {
            let Some(supplied) = bearer_token(req.authorization.as_deref()) else {
                return json_response(401, r#"{"error":"missing token"}"#);
            };
            if !constant_time_eq(supplied, token) {
                return json_response(403, r#"{"error":"invalid token"}"#);
            }
            let body: DownloadBody = match serde_json::from_slice(&req.body) {
                Ok(b) => b,
                Err(e) => {
                    return json_response(400, &format!(r#"{{"error":"bad json: {e}"}}"#));
                }
            };
            let url = body.url.trim().to_string();
            let lower = url.to_ascii_lowercase();
            if !(lower.starts_with("http://") || lower.starts_with("https://")) {
                return json_response(422, r#"{"error":"only http(s) urls are accepted"}"#);
            }

            let cookies_file = match body.cookies.as_deref() {
                Some(cookies) if !cookies.is_empty() => {
                    let path = cookie_dir.join(format!("cookies-{}.txt", uuid::Uuid::new_v4()));
                    match write_netscape_cookies(&path, cookies) {
                        Ok(()) => Some(path),
                        Err(e) => {
                            tracing::warn!("bridge: could not write cookies file: {e}");
                            return json_response(500, r#"{"error":"could not store cookies"}"#);
                        }
                    }
                }
                _ => None,
            };

            let request = BridgeRequest {
                url,
                cookies_file,
                quality: body.quality,
                output_format: body.output_format,
            };
            if tx.send(request).await.is_err() {
                return json_response(500, r#"{"error":"app is shutting down"}"#);
            }
            json_response(202, r#"{"accepted":true}"#)
        }
        _ => json_response(404, r#"{"error":"not found"}"#),
    }
}

/// Write cookies as a Netscape-format `cookies.txt` yt-dlp accepts via
/// `--cookies` (through `CookieConfig { file }` — invariant I-7). Mode 0600
/// on unix: the file holds live session cookies.
pub fn write_netscape_cookies(path: &Path, cookies: &[CookieRecord]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = String::from("# Netscape HTTP Cookie File\n");
    for c in cookies {
        // The `#HttpOnly_` prefix is the curl/yt-dlp convention for httpOnly
        // cookies; the domain-leading-dot convention drives the
        // include-subdomains flag.
        let prefix = if c.http_only { "#HttpOnly_" } else { "" };
        let include_subdomains = if c.domain.starts_with('.') {
            "TRUE"
        } else {
            "FALSE"
        };
        let secure = if c.secure { "TRUE" } else { "FALSE" };
        let expires = c.expires.map(|e| e.max(0.0) as i64).unwrap_or(0);
        // Tabs/newlines inside values would corrupt the format — strip them.
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], "");
        out.push_str(&format!(
            "{prefix}{}\t{include_subdomains}\t{}\t{secure}\t{expires}\t{}\t{}\n",
            clean(&c.domain),
            clean(&c.path),
            clean(&c.name),
            clean(&c.value),
        ));
    }
    std::fs::write(path, out)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// The app-support directory bridge cookie files live in. Cleared on app
/// start so files from crashed sessions don't linger.
pub fn cookie_dir() -> PathBuf {
    crate::utils::get_app_support_dir().join("bridge")
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "Error",
    }
}

fn json_response(code: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        status_text(code),
        body.len(),
    )
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_32_hex_chars_and_random() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b, "two generated tokens must differ");
    }

    #[test]
    fn constant_time_eq_matches_and_rejects() {
        assert!(constant_time_eq("abc123", "abc123"));
        assert!(!constant_time_eq("abc123", "abc124"));
        assert!(!constant_time_eq("abc123", "abc12"));
        assert!(!constant_time_eq("", "x"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn host_check_accepts_loopback_only() {
        assert!(host_is_loopback("127.0.0.1:46150", 46150));
        assert!(host_is_loopback("localhost:46150", 46150));
        assert!(host_is_loopback("127.0.0.1", 46150));
        assert!(host_is_loopback("LOCALHOST", 46150));
        assert!(!host_is_loopback("evil.example.com:46150", 46150));
        assert!(!host_is_loopback("192.168.1.5:46150", 46150));
        assert!(!host_is_loopback("127.0.0.1.evil.com:46150", 46150));
    }

    #[test]
    fn netscape_file_has_header_flags_and_httponly_prefix() {
        let dir = std::env::temp_dir().join(format!("rl-bridge-cookies-{}", std::process::id()));
        let path = dir.join("cookies.txt");
        let cookies = vec![
            CookieRecord {
                name: "SID".into(),
                value: "abc\tdef".into(), // tab must be stripped
                domain: ".youtube.com".into(),
                path: "/".into(),
                secure: true,
                http_only: true,
                expires: Some(1793577600.0),
            },
            CookieRecord {
                name: "session".into(),
                value: "v".into(),
                domain: "example.com".into(),
                path: "/x".into(),
                secure: false,
                http_only: false,
                expires: None,
            },
        ];
        write_netscape_cookies(&path, &cookies).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "# Netscape HTTP Cookie File");
        assert_eq!(
            lines[1],
            "#HttpOnly_.youtube.com\tTRUE\t/\tTRUE\t1793577600\tSID\tabcdef"
        );
        assert_eq!(lines[2], "example.com\tFALSE\t/x\tFALSE\t0\tsession\tv");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "cookie file must be 0600");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bridge_ports_are_the_documented_range() {
        // The design doc and the extension's bridge-client.js both name this
        // range; a drift here silently breaks discovery.
        assert_eq!(BRIDGE_PORTS, &[46150, 46151, 46152, 46153, 46154]);
    }

    // ---- integration tests: a real server on an ephemeral-ish setup ----

    /// Serve on an OS-assigned loopback port (not the fixed range, so tests
    /// never collide with a running app or each other).
    async fn spawn_test_server(
        token: &str,
    ) -> (u16, mpsc::Receiver<BridgeRequest>, tempdir::TempDirGuard) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind test listener");
        assert!(
            listener.local_addr().unwrap().ip().is_loopback(),
            "bridge listener must be loopback"
        );
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel(8);
        let dir = tempdir::TempDirGuard::new("rl-bridge-test");
        let cookie_dir = dir.path.clone();
        let token = token.to_string();
        tokio::spawn(serve(listener, token, cookie_dir, tx));
        (port, rx, dir)
    }

    /// Minimal scoped temp dir so tests clean up after themselves.
    mod tempdir {
        use std::path::PathBuf;

        pub struct TempDirGuard {
            pub path: PathBuf,
        }

        impl TempDirGuard {
            pub fn new(prefix: &str) -> Self {
                let path = std::env::temp_dir().join(format!(
                    "{prefix}-{}-{}",
                    std::process::id(),
                    uuid::Uuid::new_v4()
                ));
                std::fs::create_dir_all(&path).unwrap();
                Self { path }
            }
        }

        impl Drop for TempDirGuard {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }

    async fn raw_request(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .expect("connect");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        String::from_utf8_lossy(&response).to_string()
    }

    fn get(path: &str, port: u16, auth: Option<&str>) -> String {
        let auth_header = auth
            .map(|t| format!("Authorization: Bearer {t}\r\n"))
            .unwrap_or_default();
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth_header}\r\n")
    }

    fn post(path: &str, _port: u16, host: &str, auth: Option<&str>, body: &str) -> String {
        let auth_header = auth
            .map(|t| format!("Authorization: Bearer {t}\r\n"))
            .unwrap_or_default();
        format!(
            "POST {path} HTTP/1.1\r\nHost: {host}\r\n{auth_header}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }

    #[tokio::test]
    async fn ping_identifies_rustloader_without_leaking_token() {
        let (port, _rx, _dir) = spawn_test_server("secret-token").await;
        let resp = raw_request(port, &get("/api/v1/ping", port, None)).await;
        assert!(resp.starts_with("HTTP/1.1 200"), "got: {resp}");
        assert!(resp.contains(r#""app":"rustloader""#));
        assert!(resp.contains(r#""paired":false"#));
        assert!(!resp.contains("secret-token"), "token must never be sent");

        let resp = raw_request(port, &get("/api/v1/ping", port, Some("secret-token"))).await;
        assert!(resp.contains(r#""paired":true"#), "got: {resp}");

        let resp = raw_request(port, &get("/api/v1/ping", port, Some("wrong"))).await;
        assert!(resp.contains(r#""paired":false"#), "got: {resp}");
    }

    #[tokio::test]
    async fn download_rejects_missing_and_wrong_token() {
        let (port, mut rx, _dir) = spawn_test_server("tok").await;
        let body = r#"{"url":"https://example.com/v"}"#;

        let host = format!("127.0.0.1:{port}");
        let resp = raw_request(port, &post("/api/v1/download", port, &host, None, body)).await;
        assert!(resp.starts_with("HTTP/1.1 401"), "got: {resp}");

        let resp = raw_request(
            port,
            &post("/api/v1/download", port, &host, Some("nope"), body),
        )
        .await;
        assert!(resp.starts_with("HTTP/1.1 403"), "got: {resp}");

        assert!(
            rx.try_recv().is_err(),
            "no request may reach the app without a valid token"
        );
    }

    #[tokio::test]
    async fn download_rejects_foreign_host_header() {
        let (port, mut rx, _dir) = spawn_test_server("tok").await;
        let resp = raw_request(
            port,
            &post(
                "/api/v1/download",
                port,
                "evil.example.com",
                Some("tok"),
                r#"{"url":"https://example.com/v"}"#,
            ),
        )
        .await;
        assert!(resp.starts_with("HTTP/1.1 403"), "got: {resp}");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn download_rejects_non_http_url() {
        let (port, mut rx, _dir) = spawn_test_server("tok").await;
        let host = format!("127.0.0.1:{port}");
        let resp = raw_request(
            port,
            &post(
                "/api/v1/download",
                port,
                &host,
                Some("tok"),
                r#"{"url":"file:///etc/passwd"}"#,
            ),
        )
        .await;
        assert!(resp.starts_with("HTTP/1.1 422"), "got: {resp}");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn download_accepts_valid_token_and_delivers_request_with_cookies() {
        let (port, mut rx, dir) = spawn_test_server("tok").await;
        let host = format!("127.0.0.1:{port}");
        let body = r#"{
            "url": "https://example.com/watch?v=1",
            "cookies": [{"name":"SID","value":"s3cr3t","domain":".example.com","path":"/","secure":true,"httpOnly":true,"expires":1793577600}],
            "quality": "720",
            "output_format": "mp4"
        }"#;
        let resp = raw_request(
            port,
            &post("/api/v1/download", port, &host, Some("tok"), body),
        )
        .await;
        assert!(resp.starts_with("HTTP/1.1 202"), "got: {resp}");
        assert!(resp.contains(r#""accepted":true"#));

        let req = rx.recv().await.expect("request delivered");
        assert_eq!(req.url, "https://example.com/watch?v=1");
        assert_eq!(req.quality.as_deref(), Some("720"));
        assert_eq!(req.output_format.as_deref(), Some("mp4"));
        let cookie_path = req.cookies_file.expect("cookies file written");
        assert!(cookie_path.starts_with(&dir.path));
        let content = std::fs::read_to_string(&cookie_path).unwrap();
        assert!(content.contains("#HttpOnly_.example.com\tTRUE\t/\tTRUE\t1793577600\tSID\ts3cr3t"));
    }

    #[tokio::test]
    async fn oversized_body_is_rejected() {
        let (port, mut rx, _dir) = spawn_test_server("tok").await;
        let host = format!("127.0.0.1:{port}");
        // Declare a too-large body; the server must refuse on the header
        // alone without reading 2 MiB.
        let request = format!(
            "POST /api/v1/download HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer tok\r\nContent-Length: {}\r\n\r\n",
            2 * 1024 * 1024
        );
        let resp = raw_request(port, &request).await;
        assert!(resp.starts_with("HTTP/1.1 413"), "got: {resp}");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn unknown_path_is_404() {
        let (port, _rx, _dir) = spawn_test_server("tok").await;
        let resp = raw_request(port, &get("/api/v1/other", port, None)).await;
        assert!(resp.starts_with("HTTP/1.1 404"), "got: {resp}");
    }

    #[tokio::test]
    async fn bind_uses_first_free_port_in_range() {
        // Occupy the first port in the range, then bind() must fall through
        // to a later one. Skip (without failing) if something unrelated
        // already holds ports in the range.
        let Ok(_blocker) =
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, BRIDGE_PORTS[0])).await
        else {
            eprintln!("skipping: port {} already in use", BRIDGE_PORTS[0]);
            return;
        };
        match bind().await {
            Ok(listener) => {
                let addr = listener.local_addr().unwrap();
                assert!(addr.ip().is_loopback());
                assert_ne!(addr.port(), BRIDGE_PORTS[0]);
                assert!(BRIDGE_PORTS.contains(&addr.port()));
            }
            Err(_) => {
                // Every port taken by other processes — acceptable outside CI.
                eprintln!("skipping: whole bridge range in use");
            }
        }
    }
}
