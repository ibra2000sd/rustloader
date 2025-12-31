# Rustloader v0.1.1 Release Notes

**Release Date**: December 31, 2025  
**Type**: Bug Fix & Security Release  
**Status**: Beta

---

## Overview

Rustloader v0.1.1 is a stability and security focused release that addresses critical bugs identified during beta testing. This release is recommended for all users.

---

## What's New

### 🐛 Bug Fixes

| Issue | Description | Severity |
|-------|-------------|----------|
| BUG-001 | Fixed application freeze during video extraction | Critical |
| BUG-004 | Pause/Resume/Cancel buttons now functional | High |
| BUG-006 | Progress bars update correctly for all downloads | High |
| BUG-007 | Files properly organized into quality folders | Medium |
| BUG-008 | UI buttons no longer disappear unexpectedly | Medium |

### 🔒 Security Improvements

- **Path Traversal Prevention**: Filenames are now thoroughly sanitized to prevent directory traversal attacks from malicious video metadata
- **Improved Input Validation**: Better handling of edge cases in user input

### ⚡ Stability Improvements

- Replaced unsafe error handling with graceful fallbacks
- Eliminated potential crash scenarios from missing data
- Enhanced logging for easier troubleshooting

---

## Installation

### Requirements
- macOS 10.15+ (Catalina or later)
- yt-dlp installed (`brew install yt-dlp`)
- ~100 MB disk space

### Upgrade Instructions
1. Download the latest release from [GitHub Releases](https://github.com/ibra2000sd/rustloader/releases)
2. Replace your existing Rustloader application
3. Your settings and download history will be preserved

---

## Known Limitations

See [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for current limitations and workarounds.

---

## Feedback & Support

- **Bug Reports**: [GitHub Issues](https://github.com/ibra2000sd/rustloader/issues)
- **Feature Requests**: [GitHub Discussions](https://github.com/ibra2000sd/rustloader/discussions)

---

## What's Next (v0.1.2)

- Further reduction of compiler warnings
- Automated test suite
- Performance benchmarking
- Windows/Linux support (planned for v0.2.0)
