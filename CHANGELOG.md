# Changelog

All notable changes to Rustloader will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-12-31

### Fixed
- **BUG-001**: Resolved mutex deadlock in video extraction that could freeze the application
- **BUG-004**: Pause/Resume/Cancel buttons now work correctly
- **BUG-006**: Progress bars now update correctly for all concurrent downloads
- **BUG-007**: Downloaded files are now properly organized into quality-based folders
- **BUG-008**: Pause buttons no longer disappear due to status string mismatches

### Security
- Fixed path traversal vulnerability in filename sanitization
- Improved input validation for filenames from external sources

### Changed
- Improved error handling with graceful fallbacks instead of panics
- Enhanced debug logging for download progress tracking
- Better directory structure validation during setup
- Reduced compiler warnings for cleaner codebase

## [0.1.0] - 2025-11-23

### Added
- Initial beta release
- Multi-threaded download engine (16 segments, 5 concurrent downloads)
- GUI interface using Iced framework
- Support for 1000+ video sites via yt-dlp
- Quality-based file organization (High-Quality, Standard, Low-Quality folders)
- Download history with SQLite persistence
- Pause/Resume/Cancel functionality
- Clipboard URL detection
- Dark theme UI
