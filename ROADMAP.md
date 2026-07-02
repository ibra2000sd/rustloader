# RUSTLOADER: MASTER ROADMAP TO v1.0

> **Last Updated**: 2026-07-02  
> **Current Version**: v0.9.0  
> **Status Legend**: ✅ Completed | 🟡 In Progress | 🔴 Not Started | ⚠️ Deferred

This document tracks the evolution of Rustloader from initial release to production-ready v1.0.

---

## Philosophy

Rustloader is built on these principles:
1. **Correctness over speed** — Downloads must complete reliably, not just start quickly
2. **User transparency** — Failures are surfaced clearly with actionable guidance
3. **Predictable concurrency** — The system never exceeds configured limits, ever
4. **Resumability** — Interrupted downloads can always be recovered

---

## v0.1.0 — Initial Release ✅

**Released**: November 2025

The foundation: a working download manager with GUI.

| Feature | Status | Notes |
|---------|--------|-------|
| Multi-threaded download engine (16 segments) | ✅ | `DownloadEngine` in `src/downloader/` |
| Iced GUI framework integration | ✅ | Dark theme, sidebar navigation |
| yt-dlp integration for extraction | ✅ | Hybrid extractor with native YouTube fallback |
| SQLite-based settings persistence | ✅ | `DatabaseManager` |
| Quality-based file organization | ✅ | `FileOrganizer`, `MetadataManager` |
| Pause/Resume/Cancel basic controls | ✅ | GUI buttons functional |
| Clipboard URL detection | ✅ | `clipboard.rs` |

---

## v0.1.1 — Stability & Security ✅

**Released**: December 2025

Bug fixes and security hardening.

| Feature | Status | Notes |
|---------|--------|-------|
| Path traversal vulnerability fix | ✅ | `sanitize_filename()` comprehensive |
| Mutex deadlock in extraction fixed | ✅ | Lock released before await |
| Progress bar synchronization fixed | ✅ | Per-task progress tracking |
| Button state consistency | ✅ | Status string matching |
| File organization directory validation | ✅ | Creates directories on demand |

---

## v0.2.0 — Actor Model Architecture ✅

**Internal milestone, not separately released**

Restructured backend for maintainability and testability.

| Feature | Status | Notes |
|---------|--------|-------|
| `BackendActor` with message loop | ✅ | `src/backend/actor.rs` |
| `BackendCommand` enum (GUI → Backend) | ✅ | ExtractInfo, StartDownload, Pause, Resume, Cancel |
| `BackendEvent` enum (Backend → GUI) | ✅ | ExtractionCompleted, DownloadProgress, etc. |
| Async channel-based communication | ✅ | `mpsc::channel` for decoupling |
| GUI subscription to backend events | ✅ | `iced::subscription::unfold` |

---

## v0.3.0 — Event Sourcing & Persistence ✅

**Internal milestone**

Queue state survives application restarts.

| Feature | Status | Notes |
|---------|--------|-------|
| `QueueEvent` enum for state changes | ✅ | TaskAdded, TaskStarted, TaskPaused, TaskCompleted, TaskFailed, TaskRemoved |
| `EventLog` with JSONL persistence | ✅ | `src/queue/events.rs`, append-only log |
| `rehydrate()` method | ✅ | Reconstructs queue state from event history |
| Corruption resilience | ✅ | Skips malformed lines, continues loading |
| Started tasks rehydrate as Paused | ✅ | Prevents auto-blast on restart |

---

## v0.4.0 — Queue Manager FSM ✅

**Internal milestone**

Formal state machine for task lifecycle.

| Feature | Status | Notes |
|---------|--------|-------|
| `TaskStatus` enum | ✅ | Queued, Downloading, Paused, Completed, Failed, Cancelled |
| `QueueManager` as single source of truth | ✅ | 1047 lines, `src/queue/manager.rs` |
| Persistent scheduler loop (`start()`) | ✅ | Runs on app runtime, not GUI callbacks |
| Locking hierarchy documented | ✅ | queue (L2) → active_downloads (L1) |
| Task lifecycle event logging | ✅ | All transitions logged to EventLog |

---

## v0.5.0 — Concurrency Hardening ✅

**Internal milestone**

Eliminated race conditions in concurrent scheduling.

### v0.5.0 Core

| Feature | Status | Notes |
|---------|--------|-------|
| Max concurrent enforcement | ✅ | Checked in `process_queue()` |
| Active downloads tracking | ✅ | `HashMap<String, DownloadHandle>` |
| Cancel signal propagation | ✅ | `cancel_tx` channel per task |
| Progress update forwarding | ✅ | Separate progress handler task |

### v0.5.1 — Atomic Pre-Registration ✅

| Feature | Status | Notes |
|---------|--------|-------|
| Atomic slot reservation | ✅ | Insert placeholder into `active_downloads` BEFORE setting status |
| Zombie defense check | ✅ | Fail tasks Downloading but not in active |
| Both locks held atomically | ✅ | `queue` + `active_downloads` in `process_queue()` |
| Rollback on registration failure | ✅ | Sets status to Failed with internal error |

---

## v0.6.0 — UX Reliability ✅

User-facing quality improvements without altering core semantics.

### Stall Detection & Feedback ✅

| Feature | Status | Notes |
|---------|--------|-------|
| `last_progress_at` timestamp | ✅ | Updated on every progress event |
| 30-second stall threshold | ✅ | `STALL_THRESHOLD_SECS = 30` |
| "⚠ Stalled" UI status | ✅ | Visual warning in download list |
| Stall warning message | ✅ | "Download appears stalled. Try restarting or canceling." |
| Restart button for stalled tasks | ✅ | `Message::RestartStalled` — pause + resume |

### Error Timeline Visibility ✅

| Feature | Status | Notes |
|---------|--------|-------|
| `error_message` field on tasks | ✅ | Stored when download fails |
| Error display in UI | ✅ | Shows "✕ Error: {message}" |
| Retry tracking | ✅ | `was_resumed_after_failure` flag |
| "Previously retried" indicator | ✅ | Shown on retried failures |

### Recovery Guidance ✅

| Feature | Status | Notes |
|---------|--------|-------|
| `FailureCategory` enum | ✅ | NetworkError, AuthError, DiskError, ParseError, UnknownError |
| `from_error()` classifier | ✅ | Keyword-based categorization |
| `recovery_hint()` per category | ✅ | User-friendly guidance text |
| Recovery hint in UI | ✅ | "💡 Check your internet..." |

### Enhanced Controls ✅

| Feature | Status | Notes |
|---------|--------|-------|
| Reset Task (cancel + re-add) | ✅ | `Message::ResetTask` |
| Dismiss Error button | ✅ | `error_dismissed` flag, `Message::DismissError` |
| Open File button | ✅ | `Message::OpenFile` with `open::that()` |
| Show in Folder button | ✅ | `Message::OpenDownloadFolder` |

---

## v0.6.x — Stress Testing & Invariants ✅

Comprehensive test coverage for concurrency properties.

| Test Category | Status | Coverage |
|---------------|--------|----------|
| **Stress Tests** (`stress_test.rs`) | ✅ | 470 lines, multiple scenarios |
| Random pause/resume (50 tasks, 100 ops) | ✅ | `stress_test_random_pause_resume` |
| Concurrent resume_all (10 parallel calls) | ✅ | `stress_test_concurrent_resume_all` |
| Rapid state transitions (50 cycles) | ✅ | `stress_test_rapid_state_transitions` |
| **Invariant A** — Concurrency bound | ✅ | ≤ max_concurrent always |
| **Invariant B** — No zombie tasks | ✅ | Downloading → must be in active |
| **Invariant C** — No phantom actives | ✅ | active → must be Downloading |
| **Invariant D** — Idempotent resume | ✅ | Multiple resumes → one task |
| Property-based testing (200 ops) | ✅ | `property_invariant_a_always_holds` |
| **Execution Tests** | ✅ | Concurrency limit, FSM transitions |
| **Persistence Tests** | ✅ | Rehydration, corruption resilience |
| **Benchmarks** | ✅ | Segment downloads, file organization |

---

## v0.7.0 — Error Surfacing Improvements 🟡

**Status**: Partially implemented, see notes

| Feature | Status | Notes |
|---------|--------|-------|
| Failure category classification | ✅ | Already in v0.6.0 |
| Recovery hints per category | ✅ | Already in v0.6.0 |
| User-friendly error messages | ✅ | `make_error_user_friendly()` |
| Error history timeline | 🔴 | Only latest error stored |
| Error analytics/aggregation | 🔴 | Not implemented |

---

## v0.8.x — Resume Semantics ✅

**Status**: Shipped across the v0.8.1 → v0.9.0 download-reliability arc

| Feature | Status | Notes |
|---------|--------|-------|
| Segment-level resume (byte-level, from written bytes) | ✅ | PR #28, `src/downloader/segment.rs` |
| Corruption guard: resume requires `206 Partial Content` | ✅ | PR #29, restart-clean on a server ignoring `Range` |
| Resume across sessions (identity-guarded) | ✅ | PR #30, `downloader::resume_guard` sidecar (URL + size + segment count) |
| Orphan `.partN`/sidecar cleanup on cancel/remove | ✅ | PR #36, `cleanup_task_artifacts` |
| TLA+ model for resume | ⚠️ | Deferred — regression tests against a real mock HTTP server instead |

Scope note: byte-level resume covers the native **segmented** path (direct
media ≥1MB on `Range`-capable servers); small files and the yt-dlp/HLS path
still restart (see `KNOWN_ISSUES.md` ISSUE-001).

---

## v0.9.0 — Cross-Platform ✅

**Status**: Shipped — first release officially supporting all three platforms

| Feature | Status | Notes |
|---------|--------|-------|
| Windows support | ✅ | Official x86_64 release binary |
| Linux support | ✅ | Official x86_64 release binary (Ubuntu 22.04+, Fedora 38+) |
| Platform-specific file handling | ✅ | `src/utils/platform.rs` (XDG on Linux, AppData on Windows) |
| CI for all platforms | ✅ | `ci.yml` matrix: ubuntu-latest, macos-latest, windows-latest on every change |

---

## v1.0.0 — Production Ready 🔴

**Status**: Not started

| Feature | Status | Notes |
|---------|--------|-------|
| All compiler warnings resolved | ✅ | Zero warnings from the crate's own code; CI enforces `clippy -D warnings` |
| Full test coverage | 🟡 | Good stress tests, limited unit tests |
| Documentation complete | 🟡 | This roadmap update |
| Performance benchmarks published | 🔴 | Benchmarks exist but not documented |
| Security audit | 🔴 | Path traversal fixed, needs full audit |
| Browser extension | 🔴 | Not started |

---

## Future (Post-v1.0)

These are aspirational features not blocking v1.0:

- **Plugin architecture** — Custom extractors
- **Distributed downloads** — Multi-server acceleration
- **Scheduling** — Download at specific times
- **Bandwidth management** — Throttling
- **Cloud integration** — Upload to storage providers

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────┐
│                     RUSTLOADER                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐    ┌─────────────────────────┐       │
│  │   Iced GUI   │◄──►│    BackendActor         │       │
│  │              │    │    (Message Loop)        │       │
│  └──────────────┘    └─────────────────────────┘       │
│         │                       │                       │
│         │ BackendEvent          │ Owns                  │
│         ▼                       ▼                       │
│  ┌──────────────┐    ┌─────────────────────────┐       │
│  │ Download UI  │    │    QueueManager         │       │
│  │ Components   │    │    (FSM + Scheduler)    │       │
│  └──────────────┘    └─────────────────────────┘       │
│                              │  │                       │
│      ┌───────────────────────┘  └──────────────┐       │
│      ▼                                         ▼       │
│  ┌─────────────┐  ┌────────────────┐  ┌───────────┐   │
│  │  EventLog   │  │ DownloadEngine │  │ Extractor │   │
│  │  (JSONL)    │  │ (Multi-thread) │  │ (yt-dlp)  │   │
│  └─────────────┘  └────────────────┘  └───────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Contributing

The roadmap is maintained as a living document. When implementing features:

1. Update relevant version section with ✅
2. Add evidence (file path, function name)
3. Update "Last Updated" date
4. Cross-reference with CHANGELOG.md
