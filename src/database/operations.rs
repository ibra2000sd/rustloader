//! Database CRUD operations

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
        .bind(&record.output_path.to_string_lossy())
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

        debug!("Saved segment {} for download {}", segment.segment_number, segment.download_id);
        Ok(())
    }

    /// Get download segments
    pub async fn get_segments(&self, download_id: &str) -> Result<Vec<SegmentRecord>> {
        let rows = sqlx::query("SELECT * FROM download_segments WHERE download_id = ? ORDER BY segment_number")
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
