# 🚀 Rustloader v0.7.0 Release Notes

**Release Date**: January 2026  
**Codename**: "Architecture Evolution"  
**Type**: Major Feature Release

---

## 📦 Download

| Platform | Architecture | Download | SHA256 |
|----------|--------------|----------|--------|
| macOS | x86_64 (Intel) | [rustloader-v0.7.0-macos-x86_64.tar.gz](#) | `pending` |
| macOS | ARM64 (M1/M2/M3) | Coming in v0.8.0 | - |
| Windows | x86_64 | Coming in v0.8.0 | - |
| Linux | x86_64 | Coming in v0.8.0 | - |

---

## 🌟 Highlights

This release represents a **major architectural evolution** of Rustloader, introducing enterprise-grade patterns while maintaining the simplicity users love.

### 🎭 Actor Model Architecture
- Dedicated `BackendActor` for async operations
- Clean message-passing between GUI and backend
- No more UI freezing during downloads

### 💾 Event Sourcing & Crash Recovery
- All queue operations saved as events
- Automatic state recovery after crashes
- Resume interrupted downloads seamlessly

### 🔒 Battle-Tested Concurrency
- Zombie task detection and cleanup
- Atomic task state transitions
- Deadlock-free locking hierarchy

### 🧪 Comprehensive Testing
- **96 unit tests** covering all modules
- **Property-based testing** for edge cases
- **Stress tests** with 200+ random operations

---

## ✨ What's New

### v0.7.0 - Test Infrastructure
- Added comprehensive stress test suite
- Property-based tests with proptest
- Execution and persistence tests
- Benchmark suite for performance tracking
- Test count: 5 → 96 (+1820%)

### v0.6.0 - UX Reliability
- Stall detection (30-second threshold)
- Error classification with recovery hints
- "Restart Stalled" functionality
- Retry tracking and indicators

### v0.5.0 - Concurrency Hardening
- Atomic pre-registration of downloads
- Zombie defense mechanism
- Documented locking hierarchy
- Idempotent resume operations

### v0.4.0 - Queue Manager FSM
- Complete state machine implementation
- Task lifecycle: Queued → Downloading → Complete/Failed
- Concurrent download limiting
- 1000+ lines of robust queue logic

### v0.3.0 - Event Sourcing
- JSONL-based event persistence
- `rehydrate()` for state reconstruction
- Crash-resistant queue management

### v0.2.0 - Actor Model
- `BackendActor` for async isolation
- `BackendCommand` / `BackendEvent` messaging
- Clean separation of concerns

---

## 📊 Quality Metrics

| Metric | v0.1.1 | v0.7.0 | Improvement |
|--------|--------|--------|-------------|
| Unit Tests | 5 | 96 | +1820% |
| Integration Tests | 0 | 2 | New |
| Stress Tests | 0 | 4 | New |
| Clippy Warnings | 59 | 17 | -71% |
| Security Vulns | 1 | 0 | -100% |

---

## 🔧 Installation

### Requirements
- macOS 11+ (Big Sur or later)
- yt-dlp installed (`brew install yt-dlp`)
- ~50MB disk space

### Quick Install
```bash
# Download
curl -LO https://github.com/ibra2000sd/rustloader/releases/download/v0.7.0/rustloader-v0.7.0-macos-arm64.tar.gz

# Verify (optional)
shasum -a 256 rustloader-v0.7.0-macos-arm64.tar.gz

# Extract
tar -xzf rustloader-v0.7.0-macos-arm64.tar.gz

# Run
./rustloader
```

### Build from Source
```bash
git clone https://github.com/ibra2000sd/rustloader.git
cd rustloader
git checkout v0.7.0
cargo build --release
./target/release/rustloader
```

---

## ⚠️ Known Limitations

| Limitation | Status | Planned Fix |
|------------|--------|-------------|
| macOS only | Current | v0.8.0 |
| ARM64 only | Current | v0.8.0 |
| Binary size 34MB | Acceptable | v0.9.0 |
| 17 clippy warnings | Minor | Ongoing |

---

## 🔄 Upgrading from v0.1.x

### Breaking Changes
**None!** v0.7.0 is fully backward compatible.

### Migration Steps
1. Download v0.7.0
2. Replace old binary
3. Launch - your settings and history are preserved

### New Features to Try
- Pause a download, close the app, reopen - it resumes!
- Watch the stall detection catch stuck downloads
- Check error messages - they now have recovery hints

---

## 🐛 Bug Fixes Since v0.1.1

- ✅ Fixed mutex deadlock in concurrent extractions
- ✅ Fixed RSA timing attack vulnerability
- ✅ Fixed pause/resume buttons not working
- ✅ Fixed integration tests for EventLog
- ✅ Fixed zombie task accumulation

---

## 🗺️ Roadmap

### v0.8.0 (Next)
- [ ] Windows x86_64 support
- [ ] Linux x86_64 support
- [ ] macOS Intel (x86_64) support
- [ ] Reduce binary size

### v0.9.0
- [ ] Format selection UI
- [ ] Playlist support
- [ ] Batch downloads

### v1.0.0
- [ ] Production-ready stability
- [ ] >80% test coverage
- [ ] Plugin architecture

---

## 🙏 Acknowledgments

- **yt-dlp team** - Video extraction
- **Iced framework** - Beautiful GUI
- **Rust community** - Amazing tooling
- **Beta testers** - Valuable feedback

---

## 📄 Links

- **Repository**: https://github.com/ibra2000sd/rustloader
- **Issues**: https://github.com/ibra2000sd/rustloader/issues
- **Changelog**: https://github.com/ibra2000sd/rustloader/blob/main/CHANGELOG.md
- **Documentation**: https://github.com/ibra2000sd/rustloader/wiki

---

## 📜 License

MIT License - See [LICENSE](LICENSE)

---

**Thank you for using Rustloader!** 🎬

*Made with ❤️ and Rust*
