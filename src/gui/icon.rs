//! Application icon loader for Rustloader
//! 
//! This module handles loading and converting the application icon
//! for use in the Iced window.

use iced::window;
use image::GenericImageView;

/// The application icon as embedded PNG bytes (256x256)
const ICON_BYTES: &[u8] = include_bytes!("../../assets/icons/icon_256x256.png");

/// Load the application icon for the window
/// 
/// Returns `Some(Icon)` if the icon loads successfully, `None` otherwise.
/// This function is designed to fail gracefully - if the icon cannot be loaded,
/// the application will continue without a custom icon.
pub fn load_icon() -> Option<window::Icon> {
    // Load the PNG image from embedded bytes
    let img = match image::load_from_memory(ICON_BYTES) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Warning: Failed to load application icon: {}", e);
            return None;
        }
    };
    
    // Convert to RGBA8 format
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    
    // Create the window icon
    window::icon::from_rgba(rgba.into_raw(), width, height).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_icon_bytes_not_empty() {
        assert!(!ICON_BYTES.is_empty(), "Icon bytes should not be empty");
    }
    
    #[test]
    fn test_icon_is_png() {
        // Verify PNG magic number
        assert_eq!(&ICON_BYTES[0..4], b"\x89PNG", "Icon should be a PNG file");
    }
    
    #[test]
    fn test_icon_loads_successfully() {
        let icon = load_icon();
        assert!(icon.is_some(), "Icon should load successfully");
    }
}
