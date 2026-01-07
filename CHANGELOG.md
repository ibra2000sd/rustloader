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

## [0.6.0] - 2026-01-07

### UX Reliability Release

Major improvements to user-facing reliability and error handling.

### Added
- **Stall Detection**: Downloads without progress for 30+ seconds are flagged as stalled
  - Visual "⚠ Stalled" status indicator
  - Warning message: "Download appears stalled. Try restarting or canceling."
  - "Restart" button for stalled downloads (pause + resume)
- **Error Classification**: Failures are categorized for better user guidance
  - `FailureCategory` enum: NetworkError, AuthError, DiskError, ParseError, UnknownError
  - Keyword-based classification from error messages
  - Recovery hints per category (e.g., "Check your internet connection")
- **Enhanced Error Display**: 
  - Error messages shown inline with downloads
  - "Dismiss" button to hide error without removing task
  - "(Previously retried)" indicator for failed retries
- **Task Reset**: One-click cancel + remove + re-add for failed downloads
- **Retry Tracking**: `was_resumed_after_failure` flag to identify retried tasks
- **Progress Timestamp**: `last_progress_at` field for stall detection

### Changed
- Download item component now shows contextual controls based on status
- Failed downloads show recovery guidance based on error type
- User-friendly error message transformation via `make_error_user_friendly()`

---

## [0.5.1] - 2026-01-06

### Concurrency Hardening (Atomic Pre-Registration)

Critical fix for race conditions in the download scheduler.

### Fixed
- **Race Condition**: Eliminated window where task status was `Downloading` but not yet in `active_downloads`
- **Zombie Tasks**: Added detection for orphaned Downloading tasks without active handles
- **Concurrent Resume**: Fixed duplicate task spawning when `resume_all()` called multiple times

### Added
- **Atomic Pre-Registration**: Placeholder inserted into `active_downloads` BEFORE setting status to `Downloading`
- **Zombie Defense**: `process_queue()` now fails tasks that are Downloading but not in active
- **Locking Hierarchy**: Documented lock order (queue → active_downloads) to prevent deadlocks
- **Rollback on Failure**: If pre-registration fails, task is marked as Failed

### Technical Details
- Both `queue` and `active_downloads` locks held atomically during scheduling
- Placeholder `DownloadHandle` with dummy join handles created first
- Real handles updated after engine spawned
- Invariant: `status == Downloading` implies `task_id in active_downloads`

---

## [0.5.0] - 2026-01-05

### Test Infrastructure Release

Comprehensive stress testing and invariant verification.

### Added
- **Stress Tests** (`tests/stress_test.rs`):
  - `stress_test_random_pause_resume`: 50 tasks, 100 random operations
  - `stress_test_concurrent_resume_all`: 10 parallel resume_all calls
  - `stress_test_rapid_state_transitions`: 50 add/pause/resume/cancel cycles
  - `test_strict_concurrency_bound`: 100 tasks, verify limit never exceeded
  - `test_no_task_loss`: Verify tasks never disappear during operations
- **Invariant Checks**:
  - Invariant A: `active_downloads.len() <= max_concurrent`
  - Invariant B: Downloading tasks must be in active_downloads
  - Invariant C: active_downloads entries must have Downloading status
  - Invariant D: Multiple resumes don't spawn duplicate downloads
- **Property-Based Tests**: 200 random operations maintaining invariant A
- **Execution Tests** (`tests/execution_test.rs`):
  - Concurrency limit enforcement
  - FSM state transitions (Queued → Paused → Resumed → Cancelled)
- **Persistence Tests** (`tests/persistence_test.rs`):
  - Rehydration from event log
  - Corruption resilience (skips malformed lines)
- **Benchmarks** (`benches/`):
  - Segment download performance
  - File organizer throughput

### Changed
- `proptest` added to dev-dependencies for property testing
- `rand` added to dev-dependencies for random test data

---

## [0.4.0] - 2026-01-04

### Queue Manager FSM

Formal state machine implementation for task lifecycle.

### Added
- **TaskStatus Enum**: Queued, Downloading, Paused, Completed, Failed, Cancelled
- **QueueManager** as single source of truth for all task state
- **Persistent Scheduler Loop**: `start()` runs on app runtime, polls every 500ms
- **Task Lifecycle Events**: All transitions logged to EventLog
- **Locking Hierarchy**: queue (Level 2) → active_downloads (Level 1)

### Changed
- All state mutations go through QueueManager
- GUI no longer directly modifies task state
- Progress updates stored in both queue and active_downloads

---

## [0.3.0] - 2026-01-03

### Event Sourcing & Persistence

Queue state survives application restarts.

### Added
- **QueueEvent Enum**:
  - `TaskAdded`: New task with video_info, format, output_path
  - `TaskStarted`: Download began
  - `TaskPaused`: User paused
  - `TaskResumed`: User resumed
  - `TaskCompleted`: Success with final path
  - `TaskFailed`: Failure with error message
  - `TaskRemoved`: Cancelled or cleared
- **EventLog**: JSONL append-only persistence
  - `log()`: Append event with immediate flush
  - `read_events()`: Load all events for replay
  - Corruption resilience: skips malformed lines
- **Rehydration**: `rehydrate()` method reconstructs queue from events
  - Started tasks become Paused (prevent auto-blast on restart)
  - Completed/Failed tasks preserved
  - Removed tasks excluded

### Changed
- Tasks persist across application restarts
- Event log stored in app support directory

---

## [0.2.0] - 2026-01-02

### Actor Model Architecture

Backend restructured for maintainability and testability.

### Added
- **BackendActor**: Main backend component with message loop
  - Owns QueueManager, DownloadEngine, Extractors
  - Runs on dedicated runtime
- **BackendCommand Enum** (GUI → Backend):
  - ExtractInfo, StartDownload, PauseDownload, ResumeDownload
  - CancelDownload, RemoveTask, ClearCompleted, ResumeAll, Shutdown
- **BackendEvent Enum** (Backend → GUI):
  - ExtractionStarted, ExtractionCompleted
  - DownloadStarted, DownloadProgress, DownloadCompleted, DownloadFailed
  - TaskStatusUpdated, Error
- **Monitor Loop**: Polls QueueManager, forwards events to GUI
- **GUI Subscription**: `iced::subscription::unfold` for BackendEvent stream

### Changed
- GUI no longer directly calls download functions
- All backend operations are async message-based
- Clean separation between UI and business logic

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

[Unreleased]: https://github.com/ibra2000sd/rustloader/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/ibra2000sd/rustloader/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/ibra2000sd/rustloader/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/ibra2000sd/rustloader/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ibra2000sd/rustloader/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ibra2000sd/rustloader/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ibra2000sd/rustloader/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/ibra2000sd/rustloader/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ibra2000sd/rustloader/releases/tag/v0.1.0
