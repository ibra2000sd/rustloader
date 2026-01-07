use crate::extractor::models::{Format, VideoInfo};
use anyhow::Result;
use async_trait::async_trait;

/// Core trait for all video extractors
///
/// This trait isolates the application from the specific extraction method
/// (Native, yt-dlp, WASM plugin, etc.).
#[async_trait]
pub trait Extractor: Send + Sync {
    /// Returns a unique identifier for this extractor (e.g., "native-youtube", "ytdlp-fallback")
    fn id(&self) -> &'static str;

    /// Checks if this extractor can handle the given URL
    ///
    /// This is used to route requests to the most specific high-performance extractor first.
    fn supports(&self, url: &str) -> bool;

    /// Extracts video information
    async fn extract_info(&self, url: &str) -> Result<VideoInfo>;

    /// Extracts playlist information (optional, default implementation returns an error)
    async fn extract_playlist(&self, url: &str) -> Result<Vec<VideoInfo>> {
        // Default implementation for extractors that don't support playlists
        Err(anyhow::anyhow!(
            "Playlist extraction not supported by {}",
            self.id()
        ))
    }

    /// Gets available formats (usually calls extract_info internally)
    async fn get_formats(&self, url: &str) -> Result<Vec<Format>> {
        let info = self.extract_info(url).await?;
        Ok(info.formats)
    }

    /// Resolves the direct download URL for a specific format
    async fn get_direct_url(&self, url: &str, format_id: &str) -> Result<String>;
}
