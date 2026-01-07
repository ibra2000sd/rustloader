use chrono::Utc;
use rustloader::downloader::{DownloadConfig, DownloadEngine};
use rustloader::extractor::{Format, VideoInfo};
use rustloader::queue::{DownloadTask, EventLog, QueueEvent, QueueManager, TaskStatus};
use rustloader::utils::{FileOrganizer, MetadataManager, OrganizationSettings};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::io::AsyncWriteExt; // Fix E0599 // Fix PathBuf not found

#[tokio::test]
async fn test_persistence_rehydration() {
    // 1. Setup temporary directory
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let base_dir = temp_dir.path().to_path_buf();

    // 2. Setup EventLog
    let event_log = EventLog::new(&base_dir)
        .await
        .expect("Failed to create EventLog");
    let event_log = Arc::new(event_log);

    // 3. Simulate existing events (Previous session)
    let task_id = "task-123".to_string();
    let events = vec![
        QueueEvent::TaskAdded {
            task_id: task_id.clone(),
            video_info: VideoInfo {
                id: "video-id".to_string(),
                title: "Test Video".to_string(),
                url: "https://example.com/video".to_string(),
                ..Default::default()
            },
            format: Format {
                format_id: "best".to_string(),
                ext: "mp4".to_string(),
                ..Default::default()
            },
            output_path: base_dir.join("test.mp4"),
            timestamp: Utc::now(),
        },
        QueueEvent::TaskStarted {
            task_id: task_id.clone(),
            timestamp: Utc::now(),
        },
    ];

    for event in events {
        event_log.log(event).await.expect("Failed to log event");
    }

    // 4. Create QueueManager (New session)
    let config = DownloadConfig {
        segments: 1,
        connections_per_segment: 1,
        chunk_size: 1024,
        retry_attempts: 1,
        retry_delay: std::time::Duration::from_millis(100),
        enable_resume: false,
        request_delay: std::time::Duration::from_millis(100),
    };
    let engine = DownloadEngine::new(config);
    let org_settings = OrganizationSettings::default();
    // Use a separate dir for organization to avoid cluttering test root if needed,
    // but base_dir is fine.
    // FileOrganizer::new is async
    let file_organizer = FileOrganizer::new(org_settings)
        .await
        .expect("Failed to create FileOrganizer");
    let metadata_manager = MetadataManager::new(&base_dir);

    let queue_manager = QueueManager::new(
        2,
        engine,
        file_organizer,
        metadata_manager,
        event_log, // Inject the log with pre-written events
    );

    // 5. Rehydrate
    queue_manager
        .rehydrate()
        .await
        .expect("Failed to rehydrate");

    // 6. Assertions
    let tasks = queue_manager.get_all_tasks().await;
    assert_eq!(tasks.len(), 1, "Should have 1 task rehydrated");
    let task = &tasks[0];
    assert_eq!(task.id, task_id);
    assert_eq!(task.video_info.title, "Test Video");
    // Verify status was reset to Default (Paused/Queued) logic from rehydrate
    // Logic: TaskStarted -> rehydrate -> Paused
    assert_eq!(
        task.status,
        TaskStatus::Paused,
        "Started task should be Paused on rehydration"
    );

    println!("persistence_test PASSED");
}

#[tokio::test]
async fn test_persistence_corruption_resilience() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let base_dir = temp_dir.path().to_path_buf();

    // Create pre-existing corrupt log
    let log_path = base_dir.join("events.jsonl");
    tokio::fs::create_dir_all(&base_dir).await.unwrap();

    let mut file = tokio::fs::File::create(&log_path).await.unwrap();
    file.write_all(b"{\"TaskAdded\": ... valid json ...}\n")
        .await
        .unwrap(); // Fake valid line 1 (if we wanted)

    // Let's write a mix of valid and invalid
    let task_id_valid = "valid-task-1";
    let valid_event = QueueEvent::TaskAdded {
        task_id: task_id_valid.to_string(),
        video_info: VideoInfo {
            title: "Valid Video".into(),
            ..Default::default()
        },
        format: Format::default(),
        output_path: PathBuf::from("/tmp/video.mp4"),
        timestamp: Utc::now(),
    };

    let valid_json = serde_json::to_string(&valid_event).unwrap();

    // WRITE: Garbage -> Valid -> Garbage
    file.write_all(b"this is not json\n").await.unwrap();
    file.write_all(valid_json.as_bytes()).await.unwrap();
    file.write_all(b"\n").await.unwrap();
    file.write_all(b"{ broken json \n").await.unwrap();
    file.flush().await.unwrap();
    drop(file);

    // Initialize system
    let event_log = Arc::new(EventLog::new(&base_dir).await.expect("Failed to open log"));

    // Mock dependencies
    let config = DownloadConfig {
        segments: 1,
        ..Default::default()
    };
    let engine = DownloadEngine::new(config);
    let org = FileOrganizer::new(OrganizationSettings::default())
        .await
        .unwrap();
    let meta = MetadataManager::new(&base_dir);

    let qm = QueueManager::new(1, engine, org, meta, event_log);

    // REHYDRATE
    qm.rehydrate()
        .await
        .expect("Rehydration should not fail on corrupt lines");

    // VERIFY
    let tasks = qm.get_all_tasks().await;
    assert_eq!(tasks.len(), 1, "Should have recovered the 1 valid task");
    assert_eq!(tasks[0].id, task_id_valid);

    println!("corruption_test PASSED");
}
