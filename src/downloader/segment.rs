//! Segment-based parallel downloading
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_assignments
)]

use crate::downloader::progress::DownloadProgress;
use anyhow::Result;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Hard safety ceiling on the *total* wall-clock time a single segment may
/// spend retrying, regardless of how much forward progress it keeps making.
/// Without this, a host that serves a trickle of bytes and then drops the
/// connection forever (repeating on every retry) would let a
/// progress-resets-the-budget retry loop run indefinitely. This bounds it.
const MAX_SEGMENT_RETRY_WALL_CLOCK: Duration = Duration::from_secs(300);

/// Bytes already written to a segment's part file, or 0 if the file doesn't
/// exist yet. Used to resume a retry from where the previous attempt left
/// off instead of re-downloading the segment from `start`.
async fn part_file_len(path: &Path) -> u64 {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.len())
        .unwrap_or(0)
}

/// If a `206` response carries a `Content-Range` header, confirm its start
/// offset matches the byte we asked to resume from. Servers aren't required
/// to send this header on a `206` (the existing test server doesn't), so its
/// absence is not itself a failure — this only catches a proxy that sends
/// `206` but actually started the body from a different offset than the one
/// requested.
fn content_range_start_ok(response: &reqwest::Response, expected_start: u64) -> bool {
    let Some(value) = response.headers().get(reqwest::header::CONTENT_RANGE) else {
        return true;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    // Format: "bytes <start>-<end>/<total>".
    value
        .strip_prefix("bytes ")
        .and_then(|rest| rest.split('-').next())
        .and_then(|start| start.trim().parse::<u64>().ok())
        .is_some_and(|start| start == expected_start)
}

/// Segment information
#[derive(Debug, Clone)]
pub struct Segment {
    pub id: usize,
    pub start: u64,
    pub end: u64,
    pub size: u64,
    pub path: PathBuf,
}

/// Download a single segment
pub async fn download_segment(
    client: &Client,
    url: &str,
    segment: &Segment,
    progress_tx: mpsc::Sender<SegmentProgress>,
    retry_attempts: usize,
    retry_delay: Duration,
) -> Result<()> {
    let mut attempts = 0usize;
    let overall_start = Instant::now();
    let mut last_bytes = part_file_len(&segment.path).await;

    loop {
        match download_segment_attempt(client, url, segment, &progress_tx).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let bytes_now = part_file_len(&segment.path).await;
                let made_progress = bytes_now > last_bytes;
                last_bytes = bytes_now;

                if overall_start.elapsed() >= MAX_SEGMENT_RETRY_WALL_CLOCK {
                    error!(
                        "Segment {} exceeded the {}s retry wall-clock ceiling ({} bytes written): {}",
                        segment.id,
                        MAX_SEGMENT_RETRY_WALL_CLOCK.as_secs(),
                        bytes_now,
                        e
                    );
                    return Err(e);
                }

                if made_progress {
                    // The failed attempt still made forward progress (e.g. a
                    // throttled connection that was dropped mid-transfer) —
                    // don't burn it against the retry budget. Still bounded
                    // overall by MAX_SEGMENT_RETRY_WALL_CLOCK above, so a host
                    // that trickles bytes and drops forever cannot loop
                    // indefinitely.
                    attempts = 0;
                } else if attempts >= retry_attempts {
                    error!(
                        "Segment {} download failed after {} attempts with no forward progress: {}",
                        segment.id,
                        attempts + 1,
                        e
                    );
                    return Err(e);
                } else {
                    attempts += 1;
                }

                warn!(
                    "Segment {} download failed (attempt {}, {} bytes written so far, progress={}): {}",
                    segment.id,
                    attempts,
                    bytes_now,
                    made_progress,
                    e
                );
                sleep(retry_delay).await;
            }
        }
    }
}

/// Single attempt to download a segment
async fn download_segment_attempt(
    client: &Client,
    url: &str,
    segment: &Segment,
    progress_tx: &mpsc::Sender<SegmentProgress>,
) -> Result<()> {
    let total_size = segment.end - segment.start + 1;

    // Resume from bytes a previous attempt (this run) already wrote to the
    // part file, instead of truncating and re-downloading from `start`. If
    // the existing file is larger than the segment span it can't be a valid
    // partial write for this segment (corruption) — treat it as invalid and
    // restart the segment from scratch rather than producing a bad file.
    let existing_bytes = match tokio::fs::metadata(&segment.path).await {
        Ok(meta) if meta.len() <= total_size => meta.len(),
        _ => 0,
    };

    if existing_bytes == total_size {
        // A prior attempt already wrote the full segment before failing
        // (e.g. on the final flush) — nothing left to fetch.
        info!(
            "Segment {} already complete from a prior attempt ({} bytes)",
            segment.id, existing_bytes
        );
        let _ = progress_tx
            .send(SegmentProgress {
                segment_id: segment.id,
                downloaded_bytes: existing_bytes,
                total_bytes: total_size,
                speed: 0.0,
            })
            .await;
        return Ok(());
    }

    let range_start = segment.start + existing_bytes;

    debug!(
        "Downloading segment {} (bytes {}-{}, {} bytes already written)",
        segment.id, range_start, segment.end, existing_bytes
    );

    // Create range header for the remaining span of this segment
    let range = if range_start == segment.end {
        format!("bytes={}", range_start)
    } else {
        format!("bytes={}-{}", range_start, segment.end)
    };

    // Send request with range header
    let response = client.get(url).header("Range", range).send().await?;

    if existing_bytes > 0 {
        // Resuming a partial download is only safe if the server actually
        // honored the Range request. A server/CDN/proxy that ignores Range
        // and replies 200 OK with the full body would otherwise get appended
        // onto the existing partial bytes, silently producing an oversized,
        // corrupt part file. Require 206 Partial Content; anything else means
        // the existing partial data can't be trusted, so discard it and let
        // the retry loop restart this segment from scratch (the next attempt
        // sees existing_bytes == 0 and issues a full-span request).
        let honored = response.status() == reqwest::StatusCode::PARTIAL_CONTENT
            && content_range_start_ok(&response, range_start);
        if !honored {
            let status = response.status();
            File::create(&segment.path).await?;
            return Err(anyhow::anyhow!(
                "Range not honored on resume (status {}, expected 206 Partial Content at byte {}); segment restarted",
                status,
                range_start
            ));
        }
    } else if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    // Resume by appending to the existing part file; only create/truncate
    // fresh when there's nothing valid to resume from (first attempt, or a
    // corrupt/oversized leftover file that was reset above, or a resume
    // response whose Range wasn't honored, handled above).
    let mut file = if existing_bytes > 0 {
        OpenOptions::new().append(true).open(&segment.path).await?
    } else {
        File::create(&segment.path).await?
    };
    let mut downloaded = existing_bytes;

    // Track download speed
    let start_time = Instant::now();
    let mut last_update_time = start_time;
    let mut last_downloaded = 0u64;

    // Stream response to file
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;

        downloaded += chunk.len() as u64;

        // Update progress every second
        let now = Instant::now();
        if now.duration_since(last_update_time) >= Duration::from_secs(1) {
            let elapsed = now.duration_since(start_time).as_secs_f64();
            let speed = if elapsed > 0.0 {
                downloaded as f64 / elapsed
            } else {
                0.0
            };

            // Send progress update
            if let Err(e) = progress_tx
                .send(SegmentProgress {
                    segment_id: segment.id,
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    speed,
                })
                .await
            {
                warn!(
                    "Failed to send progress update for segment {}: {}",
                    segment.id, e
                );
            }

            last_update_time = now;
            last_downloaded = downloaded;
        }
    }

    // Ensure file is flushed
    file.flush().await?;

    // Final progress update
    let elapsed = start_time.elapsed().as_secs_f64();
    let speed = if elapsed > 0.0 {
        downloaded as f64 / elapsed
    } else {
        0.0
    };

    if let Err(e) = progress_tx
        .send(SegmentProgress {
            segment_id: segment.id,
            downloaded_bytes: downloaded,
            total_bytes: total_size,
            speed,
        })
        .await
    {
        warn!(
            "Failed to send final progress update for segment {}: {}",
            segment.id, e
        );
    }

    info!(
        "Segment {} downloaded successfully ({} bytes)",
        segment.id, downloaded
    );

    Ok(())
}

/// Progress information for a segment
#[derive(Debug, Clone)]
pub struct SegmentProgress {
    pub segment_id: usize,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: f64, // bytes per second
}

/// Calculate optimal segments for a file.
///
/// Segment temp files are placed next to `output_path` (which lives in a
/// writable directory) as `<output>.partN`, rather than relative
/// `segment_N.tmp` paths. Relative paths were written into the process CWD,
/// which fails when the CWD is read-only (e.g. a macOS app launched from Finder,
/// where CWD is `/`) and collides between concurrent downloads.
pub fn calculate_segments(file_size: u64, max_segments: usize, output_path: &Path) -> Vec<Segment> {
    if file_size == 0 {
        return Vec::new();
    }

    // Determine number of segments based on file size
    let mb = 1024 * 1024;
    let segment_count = if file_size < 10 * mb as u64 {
        1
    } else if file_size < 50 * mb as u64 {
        std::cmp::min(4, max_segments.max(1))
    } else if file_size < 500 * mb as u64 {
        std::cmp::min(16, max_segments.max(1))
    } else {
        std::cmp::max(1, max_segments)
    };

    let segment_size = file_size / segment_count as u64;
    let mut segments = Vec::with_capacity(segment_count);

    for i in 0..segment_count {
        let start = i as u64 * segment_size;
        let end = if i == segment_count - 1 {
            file_size - 1
        } else {
            (i + 1) as u64 * segment_size - 1
        };

        let size = end - start + 1;

        // Place the segment file next to the output (writable, unique per
        // output, so concurrent downloads of different files don't collide).
        let mut seg_name = output_path.file_name().unwrap_or_default().to_os_string();
        seg_name.push(format!(".part{}", i));
        let path = match output_path.parent() {
            Some(parent) => parent.join(&seg_name),
            None => PathBuf::from(&seg_name),
        };

        segments.push(Segment {
            id: i,
            start,
            end,
            size,
            path,
        });
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_segments_small_file() {
        let segments = calculate_segments(1_000, 16, Path::new("/tmp/out.mp4"));
        assert!(segments.len() <= 16);
        if let Some(last) = segments.last() {
            assert_eq!(last.end, 999);
        }
    }

    #[test]
    fn test_calculate_segments_large_file() {
        let segments = calculate_segments(100_000_000, 16, Path::new("/tmp/out.mp4"));
        assert_eq!(segments.len(), 16);
    }

    #[test]
    fn test_segment_paths_derive_from_output() {
        // Segment temp files live next to the output (absolute, writable),
        // not as relative `segment_N.tmp` in the CWD. Assert via parent/file_name
        // (platform-agnostic — avoids path-separator and is_absolute differences).
        let out = Path::new("/tmp/dl/movie.mp4");
        let segments = calculate_segments(100_000_000, 16, out);
        assert_eq!(segments[0].path.parent(), out.parent());
        assert_eq!(
            segments[0].path.file_name().unwrap(),
            std::ffi::OsStr::new("movie.mp4.part0")
        );
    }

    #[test]
    fn test_segment_ranges_no_overlap() {
        let segments = calculate_segments(10_000, 4, Path::new("/tmp/out.mp4"));
        for window in segments.windows(2) {
            let first = &window[0];
            let second = &window[1];
            assert!(
                first.end < second.start,
                "segments overlap or touch incorrectly"
            );
        }
        // Ensure full coverage
        if let (Some(first), Some(last)) = (segments.first(), segments.last()) {
            assert_eq!(first.start, 0);
            assert!(last.end >= 9_999);
        }
    }
}

/// Regression tests for segment-retry resume (F-DL-002): a segment that gets
/// dropped mid-transfer must resume from its already-written bytes on retry,
/// not truncate and restart from `start`.
///
/// These use a small hand-rolled HTTP/1.1 server over `tokio::net::TcpListener`
/// rather than a mock-server crate: no new dev-dependency is needed (`tokio`
/// is already a direct dependency), and it gives precise control over
/// closing the connection mid-body to simulate a throttled/dropped
/// connection — something response-template-based mock servers don't model.
#[cfg(test)]
mod resume_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    /// Serves ranged GET requests for `body`. The FIRST request received
    /// sends only `drop_after` bytes of the requested range and then closes
    /// the connection before `Content-Length` is satisfied (simulating a
    /// throttled/dropped connection); every subsequent request is served in
    /// full.
    async fn spawn_flaky_range_server(
        body: Vec<u8>,
        drop_after: usize,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let body = Arc::new(body);
        let request_count = Arc::new(AtomicUsize::new(0));

        let handle = tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let body = Arc::clone(&body);
                let request_count = Arc::clone(&request_count);

                tokio::spawn(async move {
                    let mut buf = [0u8; 8192];
                    let mut req = Vec::new();
                    loop {
                        let n = match socket.read(&mut buf).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => n,
                        };
                        req.extend_from_slice(&buf[..n]);
                        if req.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }

                    let req_str = String::from_utf8_lossy(&req);
                    let range_start = req_str
                        .lines()
                        .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                        .and_then(|l| l.split('=').nth(1))
                        .and_then(|r| r.split('-').next())
                        .and_then(|s| s.trim().parse::<usize>().ok())
                        .unwrap_or(0)
                        .min(body.len());

                    let this_request = request_count.fetch_add(1, Ordering::SeqCst);
                    let remaining = &body[range_start..];
                    let total_len = remaining.len();
                    let send_len = if this_request == 0 {
                        drop_after.min(total_len)
                    } else {
                        total_len
                    };

                    let headers = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        total_len
                    );
                    if socket.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    let _ = socket.write_all(&remaining[..send_len]).await;
                    let _ = socket.flush().await;
                    // On the first request, the socket is dropped here with
                    // `send_len < total_len` still outstanding against the
                    // declared Content-Length — the client sees this as a
                    // dropped/truncated connection.
                });
            }
        });

        (format!("http://{}", addr), handle)
    }

    /// Serves GET requests for `body`. The FIRST request behaves like
    /// `spawn_flaky_range_server`: it sends only `drop_after` bytes of the
    /// requested range and closes the connection before `Content-Length` is
    /// satisfied (simulating a throttled/dropped connection). Every
    /// subsequent request IGNORES any `Range` header entirely and replies
    /// `200 OK` with the full body from byte 0 — simulating a CDN/proxy that
    /// ignores `Range` and returns the whole resource, which is the scenario
    /// B-DL-001 guards against.
    async fn spawn_range_ignoring_server(
        body: Vec<u8>,
        drop_after: usize,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let body = Arc::new(body);
        let request_count = Arc::new(AtomicUsize::new(0));

        let handle = tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let body = Arc::clone(&body);
                let request_count = Arc::clone(&request_count);

                tokio::spawn(async move {
                    let mut buf = [0u8; 8192];
                    let mut req = Vec::new();
                    loop {
                        let n = match socket.read(&mut buf).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => n,
                        };
                        req.extend_from_slice(&buf[..n]);
                        if req.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }

                    let this_request = request_count.fetch_add(1, Ordering::SeqCst);

                    if this_request == 0 {
                        let total_len = body.len();
                        let send_len = drop_after.min(total_len);
                        let headers = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                            total_len
                        );
                        if socket.write_all(headers.as_bytes()).await.is_err() {
                            return;
                        }
                        let _ = socket.write_all(&body[..send_len]).await;
                        let _ = socket.flush().await;
                    } else {
                        // Range header (if any) is ignored: reply 200 with
                        // the full body, starting at byte 0.
                        let headers = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        if socket.write_all(headers.as_bytes()).await.is_err() {
                            return;
                        }
                        let _ = socket.write_all(&body).await;
                        let _ = socket.flush().await;
                    }
                });
            }
        });

        (format!("http://{}", addr), handle)
    }

    #[tokio::test]
    async fn test_download_segment_resumes_after_mid_stream_drop() {
        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 256) as u8).collect();
        let drop_after = 50_000usize;
        let (base_url, _server) = spawn_flaky_range_server(body.clone(), drop_after).await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("out.mp4.part0");
        let segment = Segment {
            id: 0,
            start: 0,
            end: (body.len() - 1) as u64,
            size: body.len() as u64,
            path: path.clone(),
        };

        let client = Client::new();
        let (tx, mut rx) = mpsc::channel(100);
        let rx_task = tokio::spawn(async move {
            let mut last = None;
            while let Some(p) = rx.recv().await {
                last = Some(p);
            }
            last
        });

        let result = download_segment(
            &client,
            &base_url,
            &segment,
            tx,
            3,
            Duration::from_millis(10),
        )
        .await;

        assert!(
            result.is_ok(),
            "expected the throttled-then-resumed segment to succeed: {:?}",
            result
        );

        let final_bytes = tokio::fs::read(&path).await.expect("read part file");
        assert_eq!(
            final_bytes.len(),
            body.len(),
            "part file must be byte-complete (no gap/overlap at the resume boundary)"
        );
        assert_eq!(
            final_bytes, body,
            "part file must be byte-identical to a clean, undropped download"
        );

        let last_progress = rx_task
            .await
            .expect("progress task")
            .expect("at least one progress update");
        assert_eq!(
            last_progress.downloaded_bytes,
            body.len() as u64,
            "progress must reach 100% of the segment, not regress or double-count on resume"
        );
    }

    #[tokio::test]
    async fn test_download_segment_happy_path_no_drop() {
        // drop_after >= body.len() means the first request is already served
        // in full — proves the resume path doesn't change first-attempt
        // (fresh-file, full-span) behaviour.
        let body: Vec<u8> = (0..20_000u32).map(|i| (i % 256) as u8).collect();
        let (base_url, _server) = spawn_flaky_range_server(body.clone(), body.len()).await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("out.mp4.part0");
        let segment = Segment {
            id: 0,
            start: 0,
            end: (body.len() - 1) as u64,
            size: body.len() as u64,
            path: path.clone(),
        };

        let client = Client::new();
        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = download_segment(
            &client,
            &base_url,
            &segment,
            tx,
            3,
            Duration::from_millis(10),
        )
        .await;
        assert!(result.is_ok());

        let final_bytes = tokio::fs::read(&path).await.expect("read part file");
        assert_eq!(
            final_bytes, body,
            "happy-path (no drop) output must be unchanged"
        );
    }

    #[tokio::test]
    async fn test_download_segment_fails_when_never_makes_progress() {
        // A listener that accepts and immediately closes every connection:
        // no bytes are ever written, so every attempt makes zero forward
        // progress. The retry budget must still exhaust and the segment must
        // fail — this is the genuine-unrecoverable path the engine's `break`
        // (which aborts the whole download) still relies on.
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                drop(socket);
            }
        });

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("out.mp4.part0");
        let segment = Segment {
            id: 0,
            start: 0,
            end: 999,
            size: 1000,
            path,
        };

        let client = Client::new();
        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = download_segment(
            &client,
            &format!("http://{}", addr),
            &segment,
            tx,
            2,
            Duration::from_millis(5),
        )
        .await;

        assert!(
            result.is_err(),
            "a segment that never makes any forward progress must still fail after exhausting retries"
        );
        handle.abort();
    }

    /// Regression test for B-DL-001: PR #28's resume path appended any 2xx
    /// response onto the existing part file, including a `200 OK` from a
    /// server/proxy that ignored the `Range` header. That silently produced
    /// an oversized, corrupt part file with no error raised. The fix
    /// requires `206 Partial Content` to resume-append; a non-206 response
    /// on resume must truncate the stale partial and restart the segment
    /// fresh instead.
    #[tokio::test]
    async fn test_resume_restarts_when_server_ignores_range() {
        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 256) as u8).collect();
        let drop_after = 50_000usize;
        let (base_url, _server) = spawn_range_ignoring_server(body.clone(), drop_after).await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("out.mp4.part0");
        let segment = Segment {
            id: 0,
            start: 0,
            end: (body.len() - 1) as u64,
            size: body.len() as u64,
            path: path.clone(),
        };

        let client = Client::new();
        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = download_segment(
            &client,
            &base_url,
            &segment,
            tx,
            3,
            Duration::from_millis(10),
        )
        .await;

        assert!(
            result.is_ok(),
            "expected the segment to recover by restarting fresh after the range-ignored resume: {:?}",
            result
        );

        let final_bytes = tokio::fs::read(&path).await.expect("read part file");
        assert_eq!(
            final_bytes.len(),
            body.len(),
            "part file must be exactly the segment size, not oversized from an appended full-body response"
        );
        assert_eq!(
            final_bytes, body,
            "part file must be byte-identical to the source body, not corrupt"
        );
    }
}
