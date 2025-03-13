// src/promo.rs
use rand::Rng;

/// Promotional messages for the free version
pub struct DownloadPromo {
    download_messages: Vec<String>,
    completion_messages: Vec<String>,
}

impl DownloadPromo {
    pub fn new() -> Self {
        Self {
            download_messages: vec![
                "⚡ Downloads would be 5X faster with Rustloader Pro! ⚡".to_string(),
                "🎬 Rustloader Pro supports 4K and 8K video quality! 🎬".to_string(),
                "🤖 AI-powered features available in Rustloader Pro! 🤖".to_string(),
            ],
            completion_messages: vec![
                "✨ Enjoy your download! Upgrade to Pro for even better quality: rustloader.com/pro ✨".to_string(),
                "🚀 Rustloader Pro removes ads and daily limits. Learn more: rustloader.com/pro 🚀".to_string(),
                "💎 Thanks for using Rustloader! Upgrade to Pro for 4K/8K quality: rustloader.com/pro 💎".to_string(),
            ],
        }
    }
    
    pub fn get_random_download_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.download_messages.len());
        &self.download_messages[idx]
    }
    
    pub fn get_random_completion_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.completion_messages.len());
        &self.completion_messages[idx]
    }
}