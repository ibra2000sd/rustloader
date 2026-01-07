//! macOS application bundle path resolution
//!
//! This module provides robust path resolution that works correctly whether the application
//! is launched from Terminal, Finder, Dock, Spotlight, or from the macOS app bundle.
//!
//! **Why this is necessary:**
//! When launched from Terminal, the current working directory (cwd) is usually the project
//! directory or user's home. However, when launched from Finder/Dock/Spotlight via LaunchServices,
//! the cwd is "/" (root), and relative paths fail silently or resolve to wrong locations.
//!
//! This module resolves all paths using standard macOS conventions:
//! - Application Support: ~/Library/Application Support/Rustloader/
//! - Downloads: ~/Downloads/ (never relative paths like ./downloads)
//! - User Home: Explicitly resolved from environment or pwd module

use std::path::PathBuf;
use tracing::{debug, warn};

/// Get the application support directory for Rustloader.
///
/// Returns: `$HOME/Library/Application Support/Rustloader/`
///
/// This is the macOS standard location for application data, preferences, and databases.
/// Creates the directory if it doesn't exist.
pub fn get_app_support_dir() -> PathBuf {
    let dir = dirs::preference_dir()
        .and_then(|parent| {
            // dirs::preference_dir() returns ~/Library/Preferences on macOS
            // We want ~/Library/Application Support instead
            parent
                .parent()
                .map(|lib| lib.join("Application Support").join("Rustloader"))
        })
        .or_else(|| {
            // Fallback: explicitly construct from home directory
            dirs::home_dir().map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join("Rustloader")
            })
        })
        .unwrap_or_else(|| {
            // Last resort: use /tmp (should never happen on a properly configured macOS system)
            PathBuf::from("/tmp/Rustloader")
        });

    // Ensure directory exists
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!(
            "Warning: Failed to create app support directory {:?}: {}",
            dir, e
        );
        eprintln!("Will attempt to use the directory anyway");
    }

    debug!("App support directory: {:?}", dir);
    dir
}

/// Get the database path for Rustloader.
///
/// Returns: `$HOME/Library/Application Support/Rustloader/rustloader.db`
pub fn get_database_path() -> PathBuf {
    get_app_support_dir().join("rustloader.db")
}

/// Get the downloads directory.
///
/// Returns: `$HOME/Downloads/` on macOS
///
/// This uses the platform-standard Downloads directory, never relative paths.
/// Falls back to ~/Downloads if the standard lookup fails.
pub fn get_downloads_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
        .unwrap_or_else(|| {
            warn!("Could not determine Downloads directory, using /tmp");
            PathBuf::from("/tmp")
        })
}

/// Get a path for a file in the downloads directory.
///
/// # Arguments
/// * `filename` - The name of the file (e.g., "my_video.mp4")
///
/// Returns: `$HOME/Downloads/{filename}`
pub fn get_download_file_path(filename: &str) -> PathBuf {
    get_downloads_dir().join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_support_dir_is_not_relative() {
        let path = get_app_support_dir();
        assert!(path.is_absolute(), "App support dir must be absolute path");
        assert!(
            path.to_string_lossy()
                .contains("Library/Application Support"),
            "Path must follow macOS convention"
        );
    }

    #[test]
    fn test_database_path_is_not_relative() {
        let path = get_database_path();
        assert!(path.is_absolute(), "Database path must be absolute path");
        assert!(
            path.to_string_lossy().ends_with("rustloader.db"),
            "Database path must end with rustloader.db"
        );
    }

    #[test]
    fn test_downloads_dir_is_not_relative() {
        let path = get_downloads_dir();
        assert!(path.is_absolute(), "Downloads dir must be absolute path");
        // Could contain Downloads or /tmp depending on system
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("Downloads") || path_str.contains("/tmp"),
            "Path should point to Downloads or tmp"
        );
    }

    #[test]
    fn test_download_file_path_is_not_relative() {
        let path = get_download_file_path("test_video.mp4");
        assert!(path.is_absolute(), "Download file path must be absolute");
        assert!(
            path.to_string_lossy().ends_with("test_video.mp4"),
            "Path must include filename"
        );
    }

    #[test]
    fn test_app_support_dir_is_accessible() {
        // This test verifies the directory can be created
        let dir = get_app_support_dir();
        assert!(
            std::fs::metadata(&dir).is_ok(),
            "App support dir should be accessible after creation"
        );
    }
}
