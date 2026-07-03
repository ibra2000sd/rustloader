//! B-GUI-003 acceptance — the GUI quality choice must constrain the download.
//!
//! Drives the exact GUI backend path (ExtractInfo → StartDownload with a
//! `VideoQuality`, no explicit format id) against a real URL with real
//! network and a real yt-dlp, so it is `#[ignore]`d and run manually, once
//! per quality, under an isolated `HOME`:
//!
//! ```sh
//! HOME=/tmp/rl-q-best  RL_QUALITY=best cargo test --test quality_acceptance -- --ignored --nocapture
//! HOME=/tmp/rl-q-480   RL_QUALITY=480  cargo test --test quality_acceptance -- --ignored --nocapture
//! ```
//!
//! then ffprobe the file under `$HOME/Downloads` and compare heights across
//! runs. `RL_TEST_URL` overrides the default test video.

use rustloader::backend::{BackendActor, BackendCommand, BackendEvent};
use rustloader::database::{initialize_database, DatabaseManager};
use rustloader::utils::config::VideoQuality;
use rustloader::utils::AppSettings;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

const EXTRACT_TIMEOUT: Duration = Duration::from_secs(300);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);

#[tokio::test]
#[ignore = "real network + yt-dlp; run manually per quality (see module doc)"]
async fn downloads_at_selected_quality() {
    // Surface backend/engine logs (which format was selected and which `-f`
    // spec the yt-dlp fallback ran with) when RUST_LOG is set. try_init: a
    // second in-process run must not panic on the existing subscriber.
    let _ = tracing_subscriber::fmt::try_init();

    let url = std::env::var("RL_TEST_URL")
        .unwrap_or_else(|_| "https://www.youtube.com/watch?v=aqz-KE-bpKQ".to_string());
    let quality_arg = std::env::var("RL_QUALITY").unwrap_or_else(|_| "best".to_string());
    let quality = match quality_arg.as_str() {
        "best" => VideoQuality::Best,
        "worst" => VideoQuality::Worst,
        h => VideoQuality::Specific(h.to_string()),
    };

    let home = PathBuf::from(std::env::var("HOME").expect("HOME must be set"));
    let dl_dir = home.join("Downloads");
    std::fs::create_dir_all(&dl_dir).expect("create download dir");

    let db_url = format!(
        "sqlite://{}?mode=rwc",
        home.join("quality-acceptance.db").display()
    );
    let pool = initialize_database(&db_url).await.expect("init db");
    let db = Arc::new(DatabaseManager::new(pool));

    let (cmd_tx, cmd_rx) = mpsc::channel::<BackendCommand>(64);
    let (evt_tx, mut evt_rx) = mpsc::channel::<BackendEvent>(1024);

    let settings = AppSettings {
        download_location: dl_dir.clone(),
        ..AppSettings::default()
    };
    let actor = BackendActor::new(settings, cmd_rx, evt_tx, db)
        .await
        .expect("backend actor");
    tokio::spawn(actor.run());

    // 1. Extract — the same command the GUI sends.
    cmd_tx
        .send(BackendCommand::ExtractInfo { url: url.clone() })
        .await
        .expect("send ExtractInfo");
    let video_info = loop {
        let evt = timeout(EXTRACT_TIMEOUT, evt_rx.recv())
            .await
            .expect("extraction timed out")
            .expect("event channel closed");
        match evt {
            BackendEvent::ExtractionCompleted(Ok(info)) => break info,
            BackendEvent::ExtractionCompleted(Err(e)) => panic!("extraction failed: {e}"),
            _ => {}
        }
    };
    println!(
        "extracted: {} ({} formats)",
        video_info.title,
        video_info.formats.len()
    );

    // 2. Download — format_id None + the quality knob, exactly like the GUI.
    let output_path = dl_dir.join(format!("quality-{quality_arg}.mp4"));
    cmd_tx
        .send(BackendCommand::StartDownload {
            video_info: Box::new(video_info),
            output_path,
            format_id: None,
            quality,
        })
        .await
        .expect("send StartDownload");

    loop {
        let evt = timeout(DOWNLOAD_TIMEOUT, evt_rx.recv())
            .await
            .expect("download timed out")
            .expect("event channel closed");
        match evt {
            BackendEvent::DownloadCompleted { file_path, .. } => {
                println!("COMPLETED quality={quality_arg} file={file_path:?}");
                break;
            }
            BackendEvent::DownloadFailed { error, .. } => panic!("download failed: {error}"),
            BackendEvent::Error(e) => panic!("backend error: {e}"),
            _ => {}
        }
    }
}
