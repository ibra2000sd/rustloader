//! Metadata management system for tracking file information
#![allow(dead_code, unused_variables, unused_mut)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Complete metadata for downloaded videos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub video_id: String,
    pub title: String,
    pub url: String,
    pub source_platform: String,
    pub duration: Option<f64>,
    pub resolution: String,
    pub format: String,
    pub file_size: u64,
    pub download_date: DateTime<Utc>,
    pub channel: Option<String>,
    pub uploader: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub quality_tier: String,
    pub content_type: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub watch_count: u32,
    pub last_accessed: DateTime<Utc>,
}

/// Manager for metadata storage and retrieval
#[derive(Clone)]
pub struct MetadataManager {
    metadata_dir: PathBuf,
}

impl MetadataManager {
    /// Initialize metadata manager with base directory
    pub fn new(base_dir: &Path) -> Self {
        Self {
            metadata_dir: base_dir.join(".metadata"),
        }
    }

    /// Ensure metadata directory exists
    async fn ensure_metadata_dir(&self) -> Result<()> {
        if !self.metadata_dir.exists() {
            fs::create_dir_all(&self.metadata_dir)
                .await
                .context("Failed to create metadata directory")?;
        }
        Ok(())
    }

    /// Save metadata to JSON file
    pub async fn save_metadata(&self, video_id: &str, metadata: &VideoMetadata) -> Result<PathBuf> {
        self.ensure_metadata_dir().await?;

        let metadata_path = self.get_metadata_path(video_id);

        eprintln!("💾 [METADATA] Saving metadata to: {:?}", metadata_path);

        let json =
            serde_json::to_string_pretty(metadata).context("Failed to serialize metadata")?;

        let mut file = fs::File::create(&metadata_path)
            .await
            .context("Failed to create metadata file")?;

        file.write_all(json.as_bytes())
            .await
            .context("Failed to write metadata")?;

        file.flush().await?;

        eprintln!("✅ [METADATA] Metadata saved successfully");
        Ok(metadata_path)
    }

    /// Load metadata from JSON file
    pub async fn load_metadata(&self, video_id: &str) -> Result<VideoMetadata> {
        let metadata_path = self.get_metadata_path(video_id);

        if !metadata_path.exists() {
            anyhow::bail!("Metadata file not found for video: {}", video_id);
        }

        let json = fs::read_to_string(&metadata_path)
            .await
            .context("Failed to read metadata file")?;

        let metadata: VideoMetadata =
            serde_json::from_str(&json).context("Failed to deserialize metadata")?;

        Ok(metadata)
    }

    /// Update existing metadata (merge with new data)
    pub async fn update_metadata(
        &self,
        video_id: &str,
        update_fn: impl FnOnce(&mut VideoMetadata),
    ) -> Result<()> {
        let mut metadata = self.load_metadata(video_id).await?;
        update_fn(&mut metadata);
        self.save_metadata(video_id, &metadata).await?;
        Ok(())
    }

    /// Delete metadata file
    pub async fn delete_metadata(&self, video_id: &str) -> Result<()> {
        let metadata_path = self.get_metadata_path(video_id);

        if metadata_path.exists() {
            fs::remove_file(&metadata_path)
                .await
                .context("Failed to delete metadata file")?;
            eprintln!("🗑️  [METADATA] Deleted metadata for: {}", video_id);
        }

        Ok(())
    }

    /// Check if metadata exists for video
    pub async fn exists(&self, video_id: &str) -> bool {
        self.get_metadata_path(video_id).exists()
    }

    /// List all metadata files
    pub async fn list_all(&self) -> Result<Vec<VideoMetadata>> {
        self.ensure_metadata_dir().await?;

        let mut entries = fs::read_dir(&self.metadata_dir)
            .await
            .context("Failed to read metadata directory")?;

        let mut metadata_list = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = fs::read_to_string(&path).await {
                    if let Ok(metadata) = serde_json::from_str::<VideoMetadata>(&json) {
                        metadata_list.push(metadata);
                    }
                }
            }
        }

        Ok(metadata_list)
    }

    /// Search metadata by title or channel
    pub async fn search(&self, query: &str) -> Result<Vec<VideoMetadata>> {
        let all_metadata = self.list_all().await?;
        let query_lower = query.to_lowercase();

        let filtered: Vec<VideoMetadata> = all_metadata
            .into_iter()
            .filter(|m| {
                m.title.to_lowercase().contains(&query_lower)
                    || m
                        .channel
                        .as_ref()
                        .is_some_and(|c| c.to_lowercase().contains(&query_lower))
                    || m
                        .uploader
                        .as_ref()
                        .is_some_and(|u| u.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(filtered)
    }

    /// Mark video as favorite
    pub async fn toggle_favorite(&self, video_id: &str) -> Result<bool> {
        let mut is_favorite = false;

        self.update_metadata(video_id, |metadata| {
            metadata.favorite = !metadata.favorite;
            is_favorite = metadata.favorite;
        })
        .await?;

        eprintln!(
            "⭐ [METADATA] Toggled favorite for {}: {}",
            video_id, is_favorite
        );
        Ok(is_favorite)
    }

    /// Update last accessed timestamp
    pub async fn update_last_accessed(&self, video_id: &str) -> Result<()> {
        self.update_metadata(video_id, |metadata| {
            metadata.last_accessed = Utc::now();
            metadata.watch_count += 1;
        })
        .await
    }

    /// Get metadata statistics
    pub async fn get_stats(&self) -> Result<MetadataStats> {
        let all_metadata = self.list_all().await?;

        let total_files = all_metadata.len();
        let total_size: u64 = all_metadata.iter().map(|m| m.file_size).sum();
        let favorites_count = all_metadata.iter().filter(|m| m.favorite).count();

        let mut by_quality = std::collections::HashMap::new();
        let mut by_source = std::collections::HashMap::new();

        for metadata in &all_metadata {
            *by_quality.entry(metadata.quality_tier.clone()).or_insert(0) += 1;
            *by_source
                .entry(metadata.source_platform.clone())
                .or_insert(0) += 1;
        }

        Ok(MetadataStats {
            total_files,
            total_size_bytes: total_size,
            favorites_count,
            by_quality,
            by_source,
        })
    }

    /// Get metadata file path for video ID
    fn get_metadata_path(&self, video_id: &str) -> PathBuf {
        self.metadata_dir.join(format!("{}.json", video_id))
    }
}

/// Statistics about metadata collection
#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub favorites_count: usize,
    pub by_quality: std::collections::HashMap<String, usize>,
    pub by_source: std::collections::HashMap<String, usize>,
}

impl MetadataStats {
    /// Get human-readable total size
    pub fn total_size_human(&self) -> String {
        let size = self.total_size_bytes as f64;

        if size >= 1_000_000_000.0 {
            format!("{:.2} GB", size / 1_000_000_000.0)
        } else if size >= 1_000_000.0 {
            format!("{:.2} MB", size / 1_000_000.0)
        } else if size >= 1_000.0 {
            format!("{:.2} KB", size / 1_000.0)
        } else {
            format!("{} bytes", self.total_size_bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metadata_roundtrip() {
        let temp_dir = std::env::temp_dir().join("rustloader_test");
        let manager = MetadataManager::new(&temp_dir);

        let metadata = VideoMetadata {
            video_id: "test123".to_string(),
            title: "Test Video".to_string(),
            url: "https://youtube.com/watch?v=test123".to_string(),
            source_platform: "YouTube".to_string(),
            duration: Some(300.0),
            resolution: "1080p".to_string(),
            format: "mp4".to_string(),
            file_size: 10_000_000,
            download_date: Utc::now(),
            channel: Some("Test Channel".to_string()),
            uploader: Some("Test Uploader".to_string()),
            description: Some("Test description".to_string()),
            thumbnail_url: None,
            quality_tier: "High-Quality".to_string(),
            content_type: "Video".to_string(),
            tags: vec!["test".to_string(), "example".to_string()],
            favorite: false,
            watch_count: 0,
            last_accessed: Utc::now(),
        };

        // Save and load
        manager.save_metadata("test123", &metadata).await.unwrap();
        let loaded = manager.load_metadata("test123").await.unwrap();

        assert_eq!(loaded.video_id, "test123");
        assert_eq!(loaded.title, "Test Video");
        assert_eq!(loaded.file_size, 10_000_000);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
    }
}
