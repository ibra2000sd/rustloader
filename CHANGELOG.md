# Changelog

All notable changes to Rustloader will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Windows and Linux support (v0.9.0)
- Segment-level resume tracking (v0.8.0)
- Browser extension integration (v1.0.0)

---

## [0.8.0] - 2026-01-08

### 🌍 Cross-Platform Support
- **NEW**: macOS ARM64 (Apple Silicon) native build
- **NEW**: Windows x86_64 support
- **NEW**: Linux x86_64 support (Ubuntu 22.04+, Fedora 38+)
- **NEW**: Automated CI/CD with GitHub Actions

### 📦 Distribution
- Pre-built binaries for all platforms
- SHA256 checksums for verification
- Reduced binary size (~20MB, down from 34MB)

### 🔧 Technical Changes
- Added platform abstraction layer (`src/utils/platform.rs`)
- Cross-platform directory handling (XDG on Linux, AppData on Windows)
- Conditional compilation for platform-specific features
- yt-dlp bundling optimization

## [0.7.0] - 2026-01-078

### 🧪 Test Infrastructure
- **Added**: Comprehensive stress test suite (`tests/stress_test.rs`)
  - 50-200 random operations per test
  - Invariant A verification: `active_downloads ≤ max_concurrent`
  - Invariant B/C: No zombie tasks, no duplicate task IDs
  - Invariant D: Idempotent resume verification
- **Added**: Property-based tests using proptest
  - `property_invariant_a_always_holds()` - 200 random operations
- **Added**: Execution tests (`tests/execution_test.rs`)
  - Concurrency limit verification
  - FSM state transition tests
- **Added**: Persistence tests (`tests/persistence_test.rs`)
  - Rehydration correctness
  - Corruption resilience
- **Added**: Benchmark suite (`benches/`)
  - Segment download benchmarks
  - File organizer benchmarks
- **Improved**: Test count increased from 5 to 96 unit tests

### 🐛 Bug Fixes
- **Fixed**: Integration tests updated for EventLog parameter
- **Fixed**: Test assertions for immediate task scheduling

### 📊 Metrics
- Unit Tests: 96 passing
- Integration Tests: 2 passing
- Clippy Warnings: 17

---

## [0.6.0] - 2025-12-XX

### ✨ UX Reliability Features
- **Added**: Stall Detection system
  - `STALL_THRESHOLD_SECS = 30` constant
  - `last_progress_at` field in `DownloadTaskUI`
  - "⚠ Stalled" status display with "Restart" button
- **Added**: Error Classification system
  - `FailureCategory` enum: NetworkError, AuthError, DiskError, ParseError, UnknownError
  - `recovery_hint()` method for user-friendly error guidance
- **Added**: Error Dismissal UI
  - `error_dismissed` field for persistent error state
  - `Message::DismissError` for user interaction
  - "Dismiss" button in error display
- **Added**: Task Reset (Re-add) functionality
  - `Message::ResetTask` - cancels, removes, and re-adds URL
  - Fresh extraction for failed downloads
- **Added**: Retry Tracking
  - `was_resumed_after_failure` field
  - "Retrying..." status indicator
  - "(Previously retried)" UI indicator
- **Added**: RestartStalled action
  - `Message::RestartStalled` - pause + resume sequence
  - Restarts download engine for stalled tasks

### 🎨 User Interface
- Improved error messages with actionable hints
- Visual indicators for stalled and retrying states
- Better feedback for recovery actions

---

## [0.5.0] - 2025-12-XX

### 🔒 Concurrency Hardening
- **Added**: Atomic Pre-Registration in `process_queue()`
  - Locks both `queue` and `active_downloads` atomically
  - Inserts placeholder into active BEFORE setting status to Downloading
  - Prevents race conditions in task state transitions
- **Added**: Zombie Defense mechanism
  - Explicit check for Downloading tasks without active handles
  - Automatic failure of orphaned tasks
  - Prevents resource leaks from crashed downloads
- **Added**: Documented Locking Hierarchy
  - "LOCKING HIERARCHY: queue (Level 2) → active (Level 1)" comments
  - Consistent lock acquisition order prevents deadlocks
- **Added**: Idempotent Resume
  - `resume_all()` and `resume_task()` properly set to Queued
  - Triggers scheduler exactly once
  - Safe to call multiple times

### 🛡️ Reliability
- Eliminated potential deadlocks in concurrent operations
- Improved task state consistency under load
- Better handling of edge cases in queue management

---

## [0.4.0] - 2025-11-XX

### 🔄 Queue Manager State Machine
- **Added**: Full FSM implementation in `QueueManager` (1000+ lines)
  - Complete state machine for download lifecycle
  - Proper state transitions with validation
- **Added**: `TaskStatus` enum with all states
  - `Queued` → `Downloading` → `Completed`
  - `Queued` → `Downloading` → `Failed(String)`
  - `Queued` → `Downloading` → `Paused` → `Queued`
  - `Queued` → `Downloading` → `Cancelled`
- **Added**: Concurrent download limiting
  - `max_concurrent` configuration
  - Automatic queue processing when slots available
- **Added**: Task lifecycle management
  - `add_task()`, `pause_task()`, `resume_task()`, `cancel_task()`
  - `get_all_tasks()`, `clear_completed()`, `remove_task()`

### 📋 Queue Features
- Automatic task scheduling based on available slots
- Progress tracking per task
- Task history with timestamps

---

## [0.3.0] - 2025-11-XX

### 💾 Event Sourcing & Persistence
- **Added**: `EventLog` struct in `src/queue/events.rs`
  - JSONL-based event persistence
  - Append-only event log for crash recovery
- **Added**: `QueueEvent` enum for all queue operations
  - TaskAdded, TaskStarted, TaskPaused
  - TaskResumed, TaskCompleted, TaskFailed, TaskCancelled
- **Added**: `rehydrate()` method for state reconstruction
  - Rebuilds queue state from event log on startup
  - Preserves download progress across app restarts
- **Added**: Event-driven architecture foundation
  - All state changes recorded as events
  - Enables future features like undo/replay

### 🔄 State Recovery
- Automatic queue restoration on app launch
- Preserves in-progress downloads after crash
- Corruption-resilient event parsing

---

## [0.2.0] - 2025-11-XX

### 🎭 Actor Model Architecture
- **Added**: `BackendActor` in `src/backend/actor.rs`
  - Dedicated actor for backend operations
  - Message-based communication with GUI
  - Async task management
- **Added**: `BackendCommand` enum in `src/backend/messages.rs`
  - ExtractVideo, StartDownload, PauseDownload
  - ResumeDownload, CancelDownload, GetStatus
- **Added**: `BackendEvent` enum for responses
  - ExtractionComplete, DownloadProgress
  - DownloadComplete, DownloadFailed, StatusUpdate
- **Improved**: Separation of concerns
  - GUI thread no longer blocks on backend operations
  - Clean message passing between components

### 🏗️ Architecture
- Moved from direct function calls to message passing
- Better error isolation between GUI and backend
- Foundation for future distributed operations

---

## [0.1.1] - 2025-12-31

### Stability & Security Release

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

---

## [0.1.0] - 2025-11-23

### Initial Beta Release

### Added
- Multi-threaded download engine (16 segments, 5 concurrent downloads)
- GUI interface using Iced framework
- Support for 1000+ video sites via yt-dlp
- Quality-based file organization (High-Quality, Standard, Low-Quality folders)
- Download history with SQLite persistence
- Pause/Resume/Cancel functionality
- Clipboard URL detection
- Dark theme UI

---

[Unreleased]: https://github.com/ibra2000sd/rustloader/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/ibra2000sd/rustloader/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/ibra2000sd/rustloader/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/ibra2000sd/rustloader/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/ibra2000sd/rustloader/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ibra2000sd/rustloader/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ibra2000sd/rustloader/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ibra2000sd/rustloader/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/ibra2000sd/rustloader/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ibra2000sd/rustloader/releases/tag/v0.1.0
