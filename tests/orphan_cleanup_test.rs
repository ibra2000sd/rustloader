//! Regression tests for orphaned `.partN` + resume-sidecar cleanup on
//! cancel/remove — the F-DL-003 hygiene spinoff.
//!
//! The load-bearing test here is the pause one: pause must LEAVE the
//! `.partN` files and the `.rustloader-resume` sidecar on disk, or
//! cross-session resume (F-DL-003/#30) breaks.

use chrono::Utc;
use rustloader::downloader::{DownloadConfig, DownloadEngine};
use rustloader::extractor::{Format, VideoInfo};
use rustloader::queue::{DownloadTask, EventLog, QueueManager, TaskStatus};
use rustloader::utils::{FileOrganizer, MetadataManager, OrganizationSettings};
use std::path::{Path, PathBuf};
use std::sync::Arc;

async fn make_queue_manager(base_dir: &Path) -> QueueManager {
    let event_log = Arc::new(EventLog::new(base_dir).await.expect("event log"));
    let config = DownloadConfig {
        segments: 1,
        ..Default::default()
    };
    let engine = DownloadEngine::new(config);
    let org = FileOrganizer::new(OrganizationSettings::default())
        .await
        .expect("file organizer");
    let meta = MetadataManager::new(base_dir);
    QueueManager::new(1, engine, org, meta, event_log)
}

fn make_task(id: &str, output_path: PathBuf) -> DownloadTask {
    DownloadTask {
        id: id.to_string(),
        video_info: VideoInfo {
            title: format!("Video {id}"),
            ..Default::default()
        },
        format: Format::default(),
        output_path,
        status: TaskStatus::Queued,
        progress: None,
        added_at: Utc::now(),
    }
}

/// Plant the artifacts a real interrupted segmented download leaves behind:
/// `<output>.partN` files and the `<output>.rustloader-resume` sidecar,
/// plus decoys that cleanup must NOT touch (the output file itself, a
/// non-numeric `.part*` name, and a different download's part).
struct PlantedFiles {
    parts: Vec<PathBuf>,
    sidecar: PathBuf,
    decoys: Vec<PathBuf>,
}

fn plant_artifacts(output_path: &Path) -> PlantedFiles {
    let dir = output_path.parent().expect("output has a parent dir");
    let name = output_path
        .file_name()
        .and_then(|n| n.to_str())
        .expect("output has a utf-8 file name");

    let parts: Vec<PathBuf> = (0..3)
        .map(|i| {
            let p = dir.join(format!("{name}.part{i}"));
            std::fs::write(&p, b"segment bytes").expect("write part file");
            p
        })
        .collect();

    let sidecar = dir.join(format!("{name}.rustloader-resume"));
    std::fs::write(&sidecar, b"{}").expect("write sidecar");

    let decoys = vec![
        output_path.to_path_buf(),
        dir.join(format!("{name}.partial")),
        dir.join(format!("other-{name}.part0")),
    ];
    for decoy in &decoys {
        std::fs::write(decoy, b"decoy bytes").expect("write decoy file");
    }

    PlantedFiles {
        parts,
        sidecar,
        decoys,
    }
}

fn assert_artifacts_gone(planted: &PlantedFiles) {
    for part in &planted.parts {
        assert!(!part.exists(), "part file should be gone: {part:?}");
    }
    assert!(
        !planted.sidecar.exists(),
        "resume sidecar should be gone: {:?}",
        planted.sidecar
    );
    for decoy in &planted.decoys {
        assert!(decoy.exists(), "decoy must be untouched: {decoy:?}");
    }
}

fn assert_artifacts_remain(planted: &PlantedFiles) {
    for part in &planted.parts {
        assert!(part.exists(), "part file must remain: {part:?}");
    }
    assert!(
        planted.sidecar.exists(),
        "resume sidecar must remain: {:?}",
        planted.sidecar
    );
    for decoy in &planted.decoys {
        assert!(decoy.exists(), "decoy must be untouched: {decoy:?}");
    }
}

#[tokio::test]
async fn test_cancel_removes_parts_and_sidecar() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base_dir = temp_dir.path().to_path_buf();
    let output_path = base_dir.join("video.mp4");
    let planted = plant_artifacts(&output_path);

    let qm = make_queue_manager(&base_dir).await;
    qm.add_task(make_task("cancel-me", output_path))
        .await
        .expect("add task");

    qm.cancel_task("cancel-me").await.expect("cancel task");

    assert_artifacts_gone(&planted);
}

#[tokio::test]
async fn test_remove_removes_parts_and_sidecar() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base_dir = temp_dir.path().to_path_buf();
    let output_path = base_dir.join("video.mp4");
    let planted = plant_artifacts(&output_path);

    let qm = make_queue_manager(&base_dir).await;
    qm.add_task(make_task("remove-me", output_path))
        .await
        .expect("add task");

    qm.remove_task("remove-me").await.expect("remove task");

    assert_artifacts_gone(&planted);
}

#[tokio::test]
async fn test_pause_keeps_parts_and_sidecar() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let base_dir = temp_dir.path().to_path_buf();
    let output_path = base_dir.join("video.mp4");
    let planted = plant_artifacts(&output_path);

    let qm = make_queue_manager(&base_dir).await;
    qm.add_task(make_task("pause-me", output_path))
        .await
        .expect("add task");

    qm.pause_task("pause-me").await.expect("pause task");

    // THE guard: pause must not destroy resumable state.
    assert_artifacts_remain(&planted);

    let tasks = qm.get_all_tasks().await;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].status, TaskStatus::Paused);
}
