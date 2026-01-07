//! Integration-style tests covering queue lifecycle and organizer flows without hitting the network.

use rustloader::downloader::{DownloadConfig, DownloadEngine};
use rustloader::extractor::{Format, VideoInfo};
use rustloader::queue::{DownloadTask, QueueManager, TaskStatus, EventLog};
use rustloader::utils::{ContentType, FileOrganizer, MetadataManager, OrganizationSettings};
use tempfile::TempDir;
use std::sync::Arc;

fn sample_format() -> Format {
    Format {
        format_id: "22".to_string(),
        ext: "mp4".to_string(),
        resolution: Some("720p".to_string()),
        filesize: Some(1_024),
        url: "https://example.com/video-720.mp4".to_string(),
        quality: None,
        fps: None,
        vcodec: None,
        acodec: None,
        format_note: None,
        width: Some(1280),
        height: Some(720),
        tbr: None,
        vbr: None,
        abr: None,
    }
}

fn sample_video() -> VideoInfo {
    VideoInfo {
        id: "vid123".to_string(),
        title: "Sample Video".to_string(),
        url: "https://example.com/watch?v=vid123".to_string(),
        direct_url: String::new(),
        duration: Some(60),
        filesize: None,
        thumbnail: None,
        uploader: Some("Uploader".to_string()),
        upload_date: None,
        formats: vec![sample_format()],
        description: None,
        view_count: None,
        like_count: None,
        extractor: Some("test".to_string()),
    }
}

#[tokio::test]
async fn queue_add_pause_resume_cancel_flow() {
    let temp = TempDir::new().expect("temp dir");
    let base_dir = temp.path().join("rustloader");
    let organizer = FileOrganizer {
        base_dir: base_dir.clone(),
        settings: OrganizationSettings::default(),
    };
    organizer
        .create_directory_structure()
        .await
        .expect("create dirs");

    let metadata = MetadataManager::new(&base_dir);
    let engine = DownloadEngine::new(DownloadConfig::default());
    let event_log = Arc::new(EventLog::new(&base_dir).await.expect("event log"));
    let queue = QueueManager::new(2, engine, organizer, metadata, event_log);

    let output = base_dir.join("Temp/test.mp4");
    let task = DownloadTask::new(sample_video(), sample_format(), output.clone());

    let task_id = queue.add_task(task).await.expect("add task");
    let tasks = queue.get_all_tasks().await;
    assert_eq!(tasks.len(), 1);

    queue.pause_task(&task_id).await.expect("pause");
    let status_after_pause = queue
        .get_all_tasks()
        .await
        .into_iter()
        .find(|t| t.id == task_id)
        .unwrap()
        .status;
    assert_eq!(status_after_pause, TaskStatus::Paused);

    queue.resume_task(&task_id).await.expect("resume");
    let status_after_resume = queue
        .get_all_tasks()
        .await
        .into_iter()
        .find(|t| t.id == task_id)
        .unwrap()
        .status;
    assert!(
        matches!(status_after_resume, TaskStatus::Queued | TaskStatus::Downloading),
        "Expected Queued or Downloading, got {:?}",
        status_after_resume
    );

    queue.cancel_task(&task_id).await.expect("cancel");
    let status_after_cancel = queue
        .get_all_tasks()
        .await
        .into_iter()
        .find(|t| t.id == task_id)
        .unwrap()
        .status;
    assert_eq!(status_after_cancel, TaskStatus::Cancelled);
}

#[tokio::test]
async fn organizer_moves_file_into_quality_folder() {
    let temp = TempDir::new().expect("temp dir");
    let base_dir = temp.path().join("rustloader");
    let organizer = FileOrganizer {
        base_dir: base_dir.clone(),
        settings: OrganizationSettings::default(),
    };
    organizer
        .create_directory_structure()
        .await
        .expect("create dirs");

    let video = sample_video();
    let source_file = base_dir.join("Temp/test_video.mp4");
    tokio::fs::create_dir_all(source_file.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&source_file, b"demo").await.unwrap();

    let organized = organizer
        .organize_file(&source_file, &video, "1080p", &ContentType::Video)
        .await
        .expect("organize file");

    assert!(organized.exists());
    assert!(organized.to_string_lossy().contains("High-Quality"));
}
