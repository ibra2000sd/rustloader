//! Database CRUD operations
#![allow(dead_code, unused_imports, unused_variables)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::{Pool, Row, Sqlite};
use std::path::PathBuf;
use tracing::{debug, error, warn};

/// Database manager
pub struct DatabaseManager {
    pool: Pool<Sqlite>,
}

impl DatabaseManager {
    /// Create new database manager
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Save download record
    pub async fn save_download(&self, record: &DownloadRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO downloads 
            (id, url, title, output_path, file_size, status, created_at, completed_at, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.url)
        .bind(&record.title)
        .bind(record.output_path.to_string_lossy())
        .bind(record.file_size.map(|v| v as i64))
        .bind(&record.status)
        .bind(record.created_at)
        .bind(record.completed_at)
        .bind(&record.error_message)
        .execute(&self.pool)
        .await?;

        debug!("Saved download record: {}", record.id);
        Ok(())
    }

    /// Get download record by ID
    pub async fn get_download(&self, id: &str) -> Result<Option<DownloadRecord>> {
        let row = sqlx::query("SELECT * FROM downloads WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(row_into_download_record(row)?)),
            None => Ok(None),
        }
    }

    /// Get all downloads
    pub async fn get_all_downloads(&self) -> Result<Vec<DownloadRecord>> {
        let rows = sqlx::query("SELECT * FROM downloads ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        let mut downloads = Vec::with_capacity(rows.len());
        for row in rows {
            downloads.push(row_into_download_record(row)?);
        }

        Ok(downloads)
    }

    /// Get downloads by status
    pub async fn get_downloads_by_status(&self, status: &str) -> Result<Vec<DownloadRecord>> {
        let rows = sqlx::query("SELECT * FROM downloads WHERE status = ? ORDER BY created_at DESC")
            .bind(status)
            .fetch_all(&self.pool)
            .await?;

        let mut downloads = Vec::with_capacity(rows.len());
        for row in rows {
            downloads.push(row_into_download_record(row)?);
        }

        Ok(downloads)
    }

    /// Delete download record
    pub async fn delete_download(&self, id: &str) -> Result<()> {
        // First delete segments
        sqlx::query("DELETE FROM download_segments WHERE download_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        // Then delete download
        sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        debug!("Deleted download record: {}", id);
        Ok(())
    }

    /// Save download segment
    pub async fn save_segment(&self, segment: &SegmentRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO download_segments 
            (download_id, segment_number, start_byte, end_byte, downloaded_bytes, completed)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&segment.download_id)
        .bind(segment.segment_number as i64)
        .bind(segment.start_byte as i64)
        .bind(segment.end_byte as i64)
        .bind(segment.downloaded_bytes as i64)
        .bind(segment.completed)
        .execute(&self.pool)
        .await?;

        debug!(
            "Saved segment {} for download {}",
            segment.segment_number, segment.download_id
        );
        Ok(())
    }

    /// Get download segments
    pub async fn get_segments(&self, download_id: &str) -> Result<Vec<SegmentRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM download_segments WHERE download_id = ? ORDER BY segment_number",
        )
        .bind(download_id)
        .fetch_all(&self.pool)
        .await?;

        let mut segments = Vec::with_capacity(rows.len());
        for row in rows {
            segments.push(row_into_segment_record(row)?);
        }

        Ok(segments)
    }

    /// Save setting
    pub async fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;

        debug!("Saved setting: {} = {}", key, value);
        Ok(())
    }

    /// Get setting
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("value")))
    }

    /// Get all settings
    pub async fn get_all_settings(&self) -> Result<Vec<SettingsRecord>> {
        let rows = sqlx::query("SELECT * FROM settings")
            .fetch_all(&self.pool)
            .await?;

        let mut settings = Vec::with_capacity(rows.len());
        for row in rows {
            settings.push(SettingsRecord {
                key: row.get("key"),
                value: row.get("value"),
            });
        }

        Ok(settings)
    }
}

/// Download record
#[derive(Debug, Clone)]
pub struct DownloadRecord {
    pub id: String,
    pub url: String,
    pub title: String,
    pub output_path: PathBuf,
    pub file_size: Option<u64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// Download segment record
#[derive(Debug, Clone)]
pub struct SegmentRecord {
    pub download_id: String,
    pub segment_number: usize,
    pub start_byte: u64,
    pub end_byte: u64,
    pub downloaded_bytes: u64,
    pub completed: bool,
}

/// Settings record
#[derive(Debug, Clone)]
pub struct SettingsRecord {
    pub key: String,
    pub value: String,
}

/// Convert database row to download record
fn row_into_download_record(row: sqlx::sqlite::SqliteRow) -> Result<DownloadRecord> {
    Ok(DownloadRecord {
        id: row.get("id"),
        url: row.get("url"),
        title: row.get("title"),
        output_path: PathBuf::from(row.get::<&str, _>("output_path")),
        file_size: row.get::<Option<i64>, _>("file_size").map(|v| v as u64),
        status: row.get("status"),
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
        error_message: row.get("error_message"),
    })
}

/// Convert database row to segment record
fn row_into_segment_record(row: sqlx::sqlite::SqliteRow) -> Result<SegmentRecord> {
    Ok(SegmentRecord {
        download_id: row.get("download_id"),
        segment_number: row.get::<i64, _>("segment_number") as usize,
        start_byte: row.get::<i64, _>("start_byte") as u64,
        end_byte: row.get::<i64, _>("end_byte") as u64,
        downloaded_bytes: row.get::<i64, _>("downloaded_bytes") as u64,
        completed: row.get("completed"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::initialize_database;

    /// Opens a fresh sqlite file at a unique temp path, mirroring the
    /// `sqlite://<path>?mode=rwc` URL format used everywhere else in the app
    /// (see `gui/app.rs`'s settings persistence).
    async fn fresh_db_url(name: &str) -> String {
        let dir =
            std::env::temp_dir().join(format!("rl-history-test-{}-{}", name, std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let db_path = dir.join("history.db");
        let _ = std::fs::remove_file(&db_path);
        format!("sqlite://{}?mode=rwc", db_path.display())
    }

    fn sample_record(id: &str, status: &str) -> DownloadRecord {
        DownloadRecord {
            id: id.to_string(),
            url: format!("https://example.com/{id}"),
            title: format!("Video {id}"),
            output_path: PathBuf::from(format!("/tmp/{id}.mp4")),
            file_size: Some(1_000_000),
            status: status.to_string(),
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    #[tokio::test]
    async fn download_history_survives_reopening_the_database() {
        // Shape-3 PR-1 acceptance criterion: history must survive a
        // simulated app restart, i.e. a brand-new connection pool against
        // the same on-disk database file, not just the same open connection.
        let db_url = fresh_db_url("restart").await;

        {
            let pool = initialize_database(&db_url).await.expect("init db");
            let db = DatabaseManager::new(pool);

            let mut completed = sample_record("task-a", "Completed");
            completed.completed_at = Some(Utc::now());
            db.save_download(&completed).await.expect("save task-a");

            let mut failed = sample_record("task-b", "Failed");
            failed.file_size = None;
            failed.completed_at = Some(Utc::now());
            failed.error_message = Some("network error".to_string());
            db.save_download(&failed).await.expect("save task-b");

            // `pool`/`db` drop here, simulating the app process exiting.
        }

        // "Restart": a brand-new pool against the same file.
        let pool = initialize_database(&db_url).await.expect("reopen db");
        let db = DatabaseManager::new(pool);
        let history = db.get_all_downloads().await.expect("get_all_downloads");

        assert_eq!(
            history.len(),
            2,
            "both records must survive reopening the database"
        );
        let a = history
            .iter()
            .find(|r| r.id == "task-a")
            .expect("task-a present after reopen");
        assert_eq!(a.status, "Completed");
        assert_eq!(a.file_size, Some(1_000_000));
        assert!(a.completed_at.is_some());

        let b = history
            .iter()
            .find(|r| r.id == "task-b")
            .expect("task-b present after reopen");
        assert_eq!(b.status, "Failed");
        assert_eq!(b.error_message.as_deref(), Some("network error"));
    }

    #[tokio::test]
    async fn status_transitions_update_in_place_not_duplicate() {
        // save_download is INSERT OR REPLACE, keyed by id -- this is what
        // lets the same task id be written on every status transition
        // (Queued -> Downloading -> Completed) without ever accumulating
        // duplicate history rows for one download.
        let db_url = fresh_db_url("transitions").await;
        let pool = initialize_database(&db_url).await.expect("init db");
        let db = DatabaseManager::new(pool);

        let id = "task-transition";
        let created_at = Utc::now();

        let mut record = sample_record(id, "Queued");
        record.created_at = created_at;
        db.save_download(&record).await.expect("save queued");

        let row = db
            .get_download(id)
            .await
            .expect("get_download")
            .expect("row exists after Queued");
        assert_eq!(row.status, "Queued");
        assert!(row.completed_at.is_none());

        record.status = "Downloading".to_string();
        db.save_download(&record).await.expect("save downloading");

        let row = db
            .get_download(id)
            .await
            .expect("get_download")
            .expect("row exists after Downloading");
        assert_eq!(row.status, "Downloading");

        record.status = "Completed".to_string();
        record.completed_at = Some(Utc::now());
        db.save_download(&record).await.expect("save completed");

        let all = db.get_all_downloads().await.expect("get_all_downloads");
        assert_eq!(
            all.len(),
            1,
            "three transitions of the same task id must leave exactly one row, not three"
        );
        assert_eq!(all[0].status, "Completed");
        assert!(all[0].completed_at.is_some());
        // created_at must be preserved across every transition (it's not part
        // of what a status update should change).
        assert_eq!(all[0].created_at.timestamp(), created_at.timestamp());
    }
}
