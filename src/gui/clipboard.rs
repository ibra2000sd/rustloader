//! Clipboard functionality

use arboard::Clipboard;

/// Get clipboard content
pub fn get_clipboard_content() -> Result<String, String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    clipboard
        .get_text()
        .map_err(|e| format!("Failed to read clipboard: {}", e))
}

/// Set clipboard content
pub fn set_clipboard_content(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to write clipboard: {}", e))
}
