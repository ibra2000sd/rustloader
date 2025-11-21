//! Database schema

use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Pool, Sqlite};
use tracing::{debug, info};

/// Initialize the database
pub async fn initialize_database(db_path: &str) -> Result<Pool<Sqlite>> {
    // Create database if it doesn't exist
    if !Sqlite::database_exists(db_path).await? {
        debug!("Creating database at: {}", db_path);
        Sqlite::create_database(db_path).await?;
    }

    // Connect to the database
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(db_path)
        .await?;

    // Run migrations
    info!("Running database migrations");
    create_tables(&pool).await?;

    Ok(pool)
}

/// Create database tables
async fn create_tables(pool: &Pool<Sqlite>) -> Result<()> {
    // Create downloads table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS downloads (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            title TEXT NOT NULL,
            output_path TEXT NOT NULL,
            file_size INTEGER,
            status TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            completed_at DATETIME,
            error_message TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create download_segments table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS download_segments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            download_id TEXT NOT NULL,
            segment_number INTEGER NOT NULL,
            start_byte INTEGER NOT NULL,
            end_byte INTEGER NOT NULL,
            downloaded_bytes INTEGER DEFAULT 0,
            completed BOOLEAN DEFAULT FALSE,
            FOREIGN KEY (download_id) REFERENCES downloads(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_segments_download ON download_segments(download_id)")
        .execute(pool)
        .await?;

    debug!("Database tables created successfully");
    Ok(())
}
