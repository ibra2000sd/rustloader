//! File organization system for automatic directory structure management

use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tokio::fs;
use crate::extractor::VideoInfo;

/// Main file organizer with configurable settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOrganizer {
    pub base_dir: PathBuf,
    pub settings: OrganizationSettings,
}

/// Organization configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    pub organize_mode: OrganizeMode,
    pub video_quality_folders: bool,
    pub date_subfolders: bool,
    pub save_metadata: bool,
    pub auto_cleanup_days: Option<u32>,
    pub favorites_enabled: bool,
    pub max_storage_gb: Option<u64>,
}

/// Organization strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrganizeMode {
    ByQuality,      // Organize by video/audio quality
    ByDate,         // Organize by download date
    BySource,       // Organize by source platform
    ByCreator,      // Organize by channel/creator
    Hybrid,         // Quality + Date (recommended)
}

/// Content type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Video,
    Audio,
    Playlist { name: String },
    Series { name: String, season: u32, episode: u32 },
}

/// Quality tier for organizing files
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityTier {
    HighQuality,    // 1080p+, 320kbps+
    Standard,       // 720p-480p, 192kbps
    LowQuality,     // <480p, <192kbps
}

impl Default for OrganizationSettings {
    fn default() -> Self {
        Self {
            organize_mode: OrganizeMode::Hybrid,
            video_quality_folders: true,
            date_subfolders: true,
            save_metadata: true,
            auto_cleanup_days: Some(180),
            favorites_enabled: true,
            max_storage_gb: Some(100),
        }
    }
}

impl FileOrganizer {
    /// Initialize organizer with user settings
    pub async fn new(settings: OrganizationSettings) -> Result<Self> {
        let base_dir = dirs::download_dir()
            .context("Failed to get downloads directory")?
            .join("Rustloader");
        
        let organizer = Self {
            base_dir,
            settings,
        };
        
        // Create directory structure on initialization
        organizer.create_directory_structure().await?;
        
        Ok(organizer)
    }
    
    /// Create base directory structure
    pub async fn create_directory_structure(&self) -> Result<()> {
        eprintln!("📁 [ORGANIZER] Creating directory structure at: {:?}", self.base_dir);
        
        // Create base directory
        fs::create_dir_all(&self.base_dir).await
            .context("Failed to create base directory")?;
        
        // Create Videos hierarchy
        let videos_dir = self.base_dir.join("Videos");
        fs::create_dir_all(videos_dir.join("High-Quality")).await?;
        fs::create_dir_all(videos_dir.join("Standard")).await?;
        fs::create_dir_all(videos_dir.join("Low-Quality")).await?;
        
        if self.settings.favorites_enabled {
            fs::create_dir_all(videos_dir.join("Favorites")).await?;
            fs::create_dir_all(videos_dir.join("Watch-Later")).await?;
        }
        
        // Create Audio hierarchy
        let audio_dir = self.base_dir.join("Audio");
        fs::create_dir_all(audio_dir.join("Music/High-320kbps")).await?;
        fs::create_dir_all(audio_dir.join("Music/Medium-192kbps")).await?;
        fs::create_dir_all(audio_dir.join("Music/Standard-128kbps")).await?;
        fs::create_dir_all(audio_dir.join("Podcasts")).await?;
        fs::create_dir_all(audio_dir.join("Audiobooks")).await?;
        
        // Create Series, Playlists, and Temp directories
        fs::create_dir_all(self.base_dir.join("Series")).await?;
        fs::create_dir_all(self.base_dir.join("Playlists")).await?;
        fs::create_dir_all(self.base_dir.join("Temp")).await?;
        
        // Create hidden metadata directory
        let metadata_dir = self.base_dir.join(".metadata");
        fs::create_dir_all(&metadata_dir).await?;
        
        // ✅ DEBUG BUG-007: Verify all directories were created
        let dirs_to_verify = vec![
            self.base_dir.join("Videos/High-Quality"),
            self.base_dir.join("Videos/Standard"),
            self.base_dir.join("Videos/Low-Quality"),
            self.base_dir.join("Audio"),
            self.base_dir.join("Temp"),
        ];

        for dir in &dirs_to_verify {
            if !dir.exists() {
                eprintln!("❌ [ORGANIZER] Directory missing: {:?}", dir);
                return Err(anyhow::anyhow!("Failed to create directory: {:?}", dir));
            }
            eprintln!("✅ [ORGANIZER] Verified: {:?}", dir);
        }
        
        eprintln!("✅ [ORGANIZER] Directory structure created and verified successfully");
        Ok(())
    }
    
    /// Determine target directory for a download
    pub fn determine_target_directory(
        &self,
        video_info: &VideoInfo,
        quality: &str,
        content_type: &ContentType,
    ) -> Result<PathBuf> {
        let mut path = self.base_dir.clone();
        
        match content_type {
            ContentType::Video => {
                path = path.join("Videos");
                
                if self.settings.video_quality_folders {
                    let tier = Self::determine_quality_tier(quality);
                    let tier_folder = match tier {
                        QualityTier::HighQuality => "High-Quality",
                        QualityTier::Standard => "Standard",
                        QualityTier::LowQuality => "Low-Quality",
                    };
                    path = path.join(tier_folder);
                }
                
                if self.settings.date_subfolders && self.settings.organize_mode == OrganizeMode::Hybrid {
                    let now = Utc::now();
                    let date_folder = format!("{}-{:02}", now.year(), now.month());
                    path = path.join(date_folder);
                }
            }
            ContentType::Audio => {
                path = path.join("Audio/Music");
                
                if self.settings.video_quality_folders {
                    let tier = Self::determine_audio_quality_tier(quality);
                    let tier_folder = match tier {
                        QualityTier::HighQuality => "High-320kbps",
                        QualityTier::Standard => "Medium-192kbps",
                        QualityTier::LowQuality => "Standard-128kbps",
                    };
                    path = path.join(tier_folder);
                }
            }
            ContentType::Playlist { name } => {
                path = path.join("Playlists").join(Self::sanitize_filename(name));
                path = path.join("videos");
            }
            ContentType::Series { name, season, episode: _ } => {
                path = path.join("Series")
                    .join(Self::sanitize_filename(name))
                    .join(format!("Season-{:02}", season));
            }
        }
        
        Ok(path)
    }
    
    /// Generate safe, standardized filename
    pub fn generate_filename(
        &self,
        video_info: &VideoInfo,
        quality: &str,
        video_id: Option<&str>,
        ext: &str,
    ) -> String {
        let mut parts = Vec::new();
        
        // Add source platform (YouTube, Vimeo, etc.)
        let source = Self::detect_source_platform(&video_info.url);
        parts.push(source);
        
        // Add sanitized title
        let title = Self::sanitize_filename(&video_info.title);
        let title = Self::truncate_title(&title, 150); // Leave room for other parts
        parts.push(title);
        
        // Add quality indicator
        parts.push(format!("[{}]", quality));
        
        // Add video ID if available (for duplicate detection)
        if let Some(id) = video_id.or_else(|| Self::extract_video_id(&video_info.url)) {
            parts.push(format!("[{}]", id));
        }
        
        // Join parts and add extension
        let filename = parts.join(" - ");
        
        format!("{}.{}", filename, ext)
    }
    
    /// Move file to organized location
    pub async fn organize_file(
        &self,
        temp_path: &Path,
        video_info: &VideoInfo,
        quality: &str,
        content_type: &ContentType,
    ) -> Result<PathBuf> {
        eprintln!("🗂️  [ORGANIZER] Organizing file from: {:?}", temp_path);
        
        // Determine target directory
        let target_dir = self.determine_target_directory(video_info, quality, content_type)?;
        
        // Create directory if it doesn't exist
        fs::create_dir_all(&target_dir).await
            .context("Failed to create target directory")?;
        
        // Generate filename
        let video_id = Self::extract_video_id(&video_info.url);
        
        // Use the extension from the actual downloaded file
        let ext = temp_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");
            
        let filename = self.generate_filename(video_info, quality, video_id, ext);
        let target_path = target_dir.join(&filename);
        
        eprintln!("📦 [ORGANIZER] Target path: {:?}", target_path);
        
        // Check if file already exists
        if target_path.exists() {
            eprintln!("⚠️  [ORGANIZER] File already exists, adding timestamp");
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let target_path = target_dir.join(format!("{}_{}", timestamp, filename));
            fs::rename(temp_path, &target_path).await
                .context("Failed to move file to organized location")?;
            return Ok(target_path);
        }
        
        // Move file atomically
        fs::rename(temp_path, &target_path).await
            .context("Failed to move file to organized location")?;
        
        eprintln!("✅ [ORGANIZER] File organized successfully");
        Ok(target_path)
    }
    
    /// Determine video quality tier from quality string
    pub fn determine_quality_tier(quality: &str) -> QualityTier {
        let quality_lower = quality.to_lowercase();
        
        // Handle special cases
        if quality_lower.contains("4k") || quality_lower.contains("2160") {
            return QualityTier::HighQuality;
        }
        
        // Extract numeric resolution if present
        let resolution = quality.chars()
            .filter(|c| c.is_numeric())
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0);
        
        if resolution >= 1080 {
            QualityTier::HighQuality
        } else if resolution >= 480 {
            QualityTier::Standard
        } else {
            QualityTier::LowQuality
        }
    }
    
    /// Determine audio quality tier from bitrate string
    fn determine_audio_quality_tier(quality: &str) -> QualityTier {
        let bitrate = quality.chars()
            .filter(|c| c.is_numeric())
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0);
        
        if bitrate >= 256 {
            QualityTier::HighQuality
        } else if bitrate >= 128 {
            QualityTier::Standard
        } else {
            QualityTier::LowQuality
        }
    }
    
    /// Sanitize filename by removing/replacing invalid characters
    pub fn sanitize_filename(name: &str) -> String {
        let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        
        name.chars()
            .map(|c| if invalid_chars.contains(&c) { '_' } else { c })
            .collect::<String>()
            .trim()
            .to_string()
    }
    
    /// Truncate title to fit within filename length limits
    fn truncate_title(title: &str, max_len: usize) -> String {
        if title.len() <= max_len {
            title.to_string()
        } else {
            format!("{}...", &title[..max_len.saturating_sub(3)])
        }
    }
    
    /// Detect source platform from URL
    pub fn detect_source_platform(url: &str) -> String {
        if url.contains("youtube.com") || url.contains("youtu.be") {
            "YouTube".to_string()
        } else if url.contains("vimeo.com") {
            "Vimeo".to_string()
        } else if url.contains("twitter.com") || url.contains("x.com") {
            "Twitter".to_string()
        } else if url.contains("soundcloud.com") {
            "SoundCloud".to_string()
        } else if url.contains("twitch.tv") {
            "Twitch".to_string()
        } else {
            "Web".to_string()
        }
    }
    
    /// Extract video ID from URL for duplicate detection
    pub fn extract_video_id(url: &str) -> Option<&str> {
        // YouTube
        if let Some(pos) = url.find("v=") {
            let id_start = pos + 2;
            let id_end = url[id_start..].find('&').unwrap_or(url.len() - id_start);
            return Some(&url[id_start..id_start + id_end]);
        }
        
        // Short YouTube URLs (youtu.be)
        if url.contains("youtu.be/") {
            if let Some(pos) = url.rfind('/') {
                let id_start = pos + 1;
                let id_end = url[id_start..].find('?').unwrap_or(url.len() - id_start);
                return Some(&url[id_start..id_start + id_end]);
            }
        }
        
        None
    }
    
    /// Get temporary directory for incomplete downloads
    pub fn get_temp_dir(&self) -> PathBuf {
        self.base_dir.join("Temp")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_filename() {
        assert_eq!(FileOrganizer::sanitize_filename("Test/Video:2024"), "Test_Video_2024");
        assert_eq!(FileOrganizer::sanitize_filename("Hello <World>"), "Hello _World_");
        assert_eq!(FileOrganizer::sanitize_filename("Normal Title"), "Normal Title");
    }
    
    #[test]
    fn test_quality_tier_detection() {
        assert_eq!(FileOrganizer::determine_quality_tier("1080p"), QualityTier::HighQuality);
        assert_eq!(FileOrganizer::determine_quality_tier("720p"), QualityTier::Standard);
        assert_eq!(FileOrganizer::determine_quality_tier("360p"), QualityTier::LowQuality);
        assert_eq!(FileOrganizer::determine_quality_tier("4K"), QualityTier::HighQuality);
    }
    
    #[test]
    fn test_detect_source_platform() {
        assert_eq!(FileOrganizer::detect_source_platform("https://www.youtube.com/watch?v=abc"), "YouTube");
        assert_eq!(FileOrganizer::detect_source_platform("https://vimeo.com/123"), "Vimeo");
        assert_eq!(FileOrganizer::detect_source_platform("https://twitter.com/user/status/123"), "Twitter");
        assert_eq!(FileOrganizer::detect_source_platform("https://example.com/video"), "Web");
    }
    
    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            FileOrganizer::extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(
            FileOrganizer::extract_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(
            FileOrganizer::extract_video_id("https://www.youtube.com/watch?v=abc123&list=xyz"),
            Some("abc123")
        );
    }
}
