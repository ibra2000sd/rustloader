use crate::extractor::models::VideoInfo;
use crate::extractor::traits::Extractor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::{info, warn};

pub struct NativeYoutubeExtractor;

impl Default for NativeYoutubeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeYoutubeExtractor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Extractor for NativeYoutubeExtractor {
    fn id(&self) -> &'static str {
        "native-youtube"
    }

    fn supports(&self, url: &str) -> bool {
        url.contains("youtube.com") || url.contains("youtu.be")
    }

    async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
        info!("NativeYoutubeExtractor invoked for: {}", url);
        // Stub: for now, we just return an error to force fallback or fail safely
        // In reality, this would use untrusted-native-extraction logic

        warn!("Native extraction not yet implemented, returning stub error");
        Err(anyhow!("Native extraction not implemented yet"))
    }

    async fn get_direct_url(&self, _url: &str, _format_id: &str) -> Result<String> {
        Err(anyhow!("Native extraction not implemented yet"))
    }
}
