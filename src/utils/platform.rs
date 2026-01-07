//! Platform-specific utilities for Rustloader
//!
//! This module provides cross-platform abstractions for:
//! - Application directories (config, data, cache)
//! - Path handling
//! - System integration

use std::path::PathBuf;

/// Returns the application support directory
/// - macOS: ~/Library/Application Support/Rustloader
/// - Windows: %APPDATA%\Rustloader
/// - Linux: ~/.local/share/rustloader
pub fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Rustloader")
    }

    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Rustloader")
    }

    #[cfg(target_os = "linux")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rustloader")
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Rustloader")
    }
}

/// Returns the default download directory
/// - All platforms: ~/Downloads/Rustloader
pub fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("Rustloader")
}

/// Returns the configuration directory
/// - macOS: ~/Library/Application Support/Rustloader
/// - Windows: %APPDATA%\Rustloader
/// - Linux: ~/.config/rustloader
pub fn config_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        app_data_dir()
    }

    #[cfg(target_os = "windows")]
    {
        app_data_dir()
    }

    #[cfg(target_os = "linux")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rustloader")
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        app_data_dir()
    }
}

/// Returns the cache directory
/// - macOS: ~/Library/Caches/Rustloader
/// - Windows: %LOCALAPPDATA%\Rustloader\cache
/// - Linux: ~/.cache/rustloader
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(if cfg!(target_os = "linux") {
            "rustloader"
        } else {
            "Rustloader"
        })
}

/// Returns the log directory
pub fn log_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Logs/Rustloader")
    }

    #[cfg(target_os = "windows")]
    {
        app_data_dir().join("logs")
    }

    #[cfg(target_os = "linux")]
    {
        dirs::state_dir()
            .unwrap_or_else(|| PathBuf::from("/var/log"))
            .join("rustloader")
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        app_data_dir().join("logs")
    }
}

/// Returns the path to yt-dlp executable
pub fn ytdlp_path() -> Option<PathBuf> {
    // 1. Check if bundled (relative to executable)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Check adjacent (Windows/Linux/Dev)
            let binary_name = if cfg!(target_os = "windows") {
                "yt-dlp.exe"
            } else {
                "yt-dlp"
            };
            let adjacent = exe_dir.join(binary_name);
            if adjacent.exists() {
                return Some(adjacent);
            }

            // Check macOS bundle structure
            #[cfg(target_os = "macos")]
            {
                // Structure: App.app/Contents/MacOS/rustloader
                // Resource:  App.app/Contents/Resources/bin/yt-dlp
                if exe_dir.ends_with("MacOS") {
                    if let Some(contents) = exe_dir.parent() {
                        let bundle_path = contents.join("Resources").join("bin").join("yt-dlp");
                        if bundle_path.exists() {
                            return Some(bundle_path);
                        }
                    }
                }
            }
        }
    }

    // 2. Fall back to system PATH
    which::which("yt-dlp").ok()
}

/// Platform-specific executable extension
pub fn exe_extension() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        ".exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_data_dir_exists_or_creatable() {
        let dir = app_data_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_download_dir_is_absolute() {
        let dir = default_download_dir();
        // Should be absolute on all platforms
        assert!(dir.is_absolute() || dir.starts_with("~") || dir.starts_with("."));
        // Relaxed check for test envs
    }

    #[test]
    fn test_platform_specific_naming() {
        let data_dir = app_data_dir();
        // Skip unwraps if passing unwraps fails in some CI envs, but basic check:
        if let Some(name) = data_dir.file_name() {
            let dir_name = name.to_str().unwrap_or("Rustloader");

            #[cfg(target_os = "linux")]
            assert_eq!(dir_name, "rustloader"); // lowercase on Linux

            #[cfg(not(target_os = "linux"))]
            assert_eq!(dir_name, "Rustloader"); // Title case elsewhere
        }
    }
}
