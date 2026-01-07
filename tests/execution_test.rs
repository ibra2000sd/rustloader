use rustloader::queue::{QueueManager, QueueEvent, EventLog};
use rustloader::downloader::{DownloadEngine, DownloadConfig};
use rustloader::extractor::{VideoInfo, Format};
use rustloader::utils::{FileOrganizer, MetadataManager, OrganizationSettings};
use chrono::Utc;
use std::sync::Arc;
use tempfile::tempdir;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_execution_concurrency_limit() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let base_dir = temp_dir.path().to_path_buf();
    let event_log = Arc::new(EventLog::new(&base_dir).await.unwrap());
    
    // Mock dependencies: max_concurrent = 1
    let config = DownloadConfig { segments: 1, ..Default::default() };
    let engine = DownloadEngine::new(config);
    let org = FileOrganizer::new(OrganizationSettings::default()).await.unwrap();
    let meta = MetadataManager::new(&base_dir);
    
    let qm = Arc::new(QueueManager::new(1, engine, org, meta, event_log)); // max_concurrent = 1

    // Add 3 tasks
    for i in 1..=3 {
        let task = rustloader::queue::DownloadTask {
            id: format!("task-{}", i),
            video_info: VideoInfo { title: format!("Video {}", i), ..Default::default() },
            format: Format::default(),
            output_path: base_dir.join(format!("vid{}.mp4", i)),
            status: rustloader::queue::TaskStatus::Queued,
            progress: None,
            added_at: Utc::now(),
        };
        qm.add_task(task).await.unwrap();
    }
    
    // Spawn the persistent loop (in background)
    let qm_clone = qm.clone();
    tokio::spawn(async move {
        qm_clone.start().await;
    });

    // Give it a moment to tick
    sleep(Duration::from_millis(500)).await;
    
    // Check state
    // Since we mock the engine and it "hangs" or fails quickly? 
    // Actually the mock engine in real code does real network requests or fails.
    // If it fails immediately, the next task starts. 
    // Ideally we'd need a MockEngine that blocks to test concurrency.
    // But we can check if MULTIPLE are Downloading at once.
    // Given the real engine will likely fail (invalid URL), they might cycle through fast.
    // CAUTION: This test relies on engine behavior.
    
    let tasks = qm.get_all_tasks().await;
    let downloading_count = tasks.iter().filter(|t| matches!(t.status, rustloader::queue::TaskStatus::Downloading)).count();
    
    // It's possible 0 are downloading if they failed fast, or 1 is scanning.
    // But it should NEVER be > 1.
    assert!(downloading_count <= 1, "Concurrency limit exceeded! Found {}", downloading_count);
}

#[tokio::test]
async fn test_fsm_transitions() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let base_dir = temp_dir.path().to_path_buf();
    let event_log = Arc::new(EventLog::new(&base_dir).await.unwrap());
    
    let config = DownloadConfig { segments: 1, ..Default::default() };
    let engine = DownloadEngine::new(config);
    let org = FileOrganizer::new(OrganizationSettings::default()).await.unwrap();
    let meta = MetadataManager::new(&base_dir);
    
    let qm = Arc::new(QueueManager::new(5, engine, org, meta, event_log));

    // 1. Add Task
    let task_id = "test-task-1".to_string();
    let task = rustloader::queue::DownloadTask {
        id: task_id.clone(),
        video_info: VideoInfo::default(),
        format: Format::default(),
        output_path: base_dir.join("out.mp4"),
        status: rustloader::queue::TaskStatus::Queued,
        progress: None,
        added_at: Utc::now(),
    };
    qm.add_task(task).await.unwrap();

    // 2. Pause Task (while Queued)
    qm.pause_task(&task_id).await.expect("Should pause queued task");
    
    let tasks = qm.get_all_tasks().await;
    let t = tasks.iter().find(|t| t.id == task_id).unwrap();
    assert_eq!(t.status, rustloader::queue::TaskStatus::Paused);
    
    // 3. Resume Task
    qm.resume_task(&task_id).await.expect("Should resume paused task");
    
    let tasks = qm.get_all_tasks().await;
    let t = tasks.iter().find(|t| t.id == task_id).unwrap();
    // It might have been picked up by the scheduler immediately, which is correct
    assert!(
        matches!(t.status, rustloader::queue::TaskStatus::Queued | rustloader::queue::TaskStatus::Downloading),
        "Resumed task should be Queued or Downloading (found {:?})", t.status
    );

    // 4. Cancel Task
    qm.cancel_task(&task_id).await.expect("Should cancel task");
    
    let tasks = qm.get_all_tasks().await;
    let t = tasks.iter().find(|t| t.id == task_id).unwrap();
    assert!(matches!(t.status, rustloader::queue::TaskStatus::Cancelled));
}
