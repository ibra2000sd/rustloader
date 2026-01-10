use crate::extractor::models::{Format, VideoInfo};
use crate::extractor::traits::Extractor;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};

/// The Hybrid Extractor Registry
///
/// This struct holds a list of available extractors and routes requests
/// to the most appropriate one based on `supports(url)`.
pub struct HybridExtractor {
    extractors: Vec<Arc<dyn Extractor>>,
    fallback: Arc<dyn Extractor>,
}

impl HybridExtractor {
    /// Create a new HybridExtractor with the given registry and fallback
    pub fn new(extractors: Vec<Arc<dyn Extractor>>, fallback: Arc<dyn Extractor>) -> Self {
        Self {
            extractors,
            fallback,
        }
    }

    /// Find the best extractor for a given URL
    fn find_extractor(&self, url: &str) -> &Arc<dyn Extractor> {
        for extractor in &self.extractors {
            if extractor.supports(url) {
                debug!("Routing to extractor: {}", extractor.id());
                return extractor;
            }
        }
        debug!("Routing to fallback extractor: {}", self.fallback.id());
        &self.fallback
    }

    /// Extract video info using the best matching strategy
    pub async fn extract_info(&self, url: &str) -> Result<VideoInfo> {
        let extractor = self.find_extractor(url);
        match extractor.extract_info(url).await {
            Ok(info) => Ok(info),
            Err(e) => {
                // If a specialized extractor fails, try the fallback!
                info!(
                    "Primary extractor {} failed: {}. Retrying with fallback...",
                    extractor.id(),
                    e
                );
                if extractor.id() != self.fallback.id() {
                    self.fallback.extract_info(url).await
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn extract_playlist(&self, url: &str) -> Result<Vec<VideoInfo>> {
        let extractor = self.find_extractor(url);
        extractor.extract_playlist(url).await
    }

    pub async fn get_formats(&self, url: &str) -> Result<Vec<Format>> {
        let extractor = self.find_extractor(url);
        extractor.get_formats(url).await
    }

    pub async fn get_direct_url(&self, url: &str, format_id: &str) -> Result<String> {
        let extractor = self.find_extractor(url);
        match extractor.get_direct_url(url, format_id).await {
            Ok(url) => Ok(url),
            Err(e) => {
                info!(
                    "Primary extractor {} failed to get direct URL: {}. Retrying with fallback...",
                    extractor.id(),
                    e
                );
                if extractor.id() != self.fallback.id() {
                    self.fallback.get_direct_url(url, format_id).await
                } else {
                    Err(e)
                }
            }
        }
    }
}
