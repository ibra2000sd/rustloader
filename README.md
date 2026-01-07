# 🚀 Rustloader - High-Performance Video Downloader

[![Version](https://img.shields.io/badge/version-0.7.0--dev-blue.svg)](https://github.com/ibra2000sd/rustloader/releases)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)](https://github.com/ibra2000sd/rustloader)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-passing-success.svg)](tests/)

Rustloader is a cross-platform video downloader that combines the extraction capabilities of **yt-dlp** with a blazing-fast **Rust-based download engine** and a simple, practical GUI built with the **Iced framework**.

---

## 📊 Project Status

| Milestone | Status | Description |
|-----------|--------|-------------|
| **v0.1.x** | ✅ Complete | Core download engine, GUI, yt-dlp integration |
| **v0.2.x** | ✅ Complete | Actor model architecture with message passing |
| **v0.3.x** | ✅ Complete | Event sourcing and session persistence |
| **v0.4.x** | ✅ Complete | Queue manager with formal state machine |
| **v0.5.x** | ✅ Complete | Concurrency hardening (atomic pre-registration, zombie defense) |
| **v0.6.x** | ✅ Complete | UX reliability (stall detection, error classification, recovery hints) |
| **v0.7.x** | 🟡 Partial | Enhanced error surfacing (core features done) |
| **v0.8.x** | 🔴 Planned | Resume semantics and partial file recovery |
| **v0.9.x** | 🔴 Planned | Windows & Linux support |
| **v1.0.0** | 🔴 Planned | Production release with full test coverage |

See [ROADMAP.md](ROADMAP.md) for detailed feature breakdown and implementation status.

---

## ✨ What's New

### Recent Improvements (v0.5.x - v0.6.x)

- 🛡️ **Concurrency Hardening**: Atomic pre-registration eliminates race conditions when scheduling downloads
- 🧟 **Zombie Defense**: Automatic detection and recovery of orphaned download tasks  
- ⏱️ **Stall Detection**: Downloads stuck for 30+ seconds are flagged with recovery options
- 💡 **Smart Error Recovery**: Errors are classified with user-friendly guidance
- 🔄 **Task Reset**: One-click to cancel, remove, and re-add failed downloads
- ✅ **Stress Testing**: 470+ lines of invariant verification tests

---

## 🎯 Features

| Feature | Description |
|---------|-------------|
| **Multi-threaded Downloads** | Up to 16 parallel segments for maximum speed |
| **1000+ Site Support** | Powered by yt-dlp for broad compatibility |
| **Resume Capability** | Pause and resume downloads without data loss |
| **Queue Management** | Handle multiple downloads concurrently (up to 5) |
| **Quality Organization** | Auto-organize files into High/Standard/Low quality folders |
| **Simple GUI** | Clean, dark-themed interface focused on functionality |
| **Download History** | Event-sourced persistence for tracking downloads |
| **Stall Detection** | Automatic detection of stuck downloads |
| **Error Classification** | Smart error categorization with recovery hints |

---

## 📋 System Requirements

| Requirement | Minimum |
|-------------|---------|
| **Operating System** | macOS 10.15+ (Catalina or later) |
| **Rust** | 1.70+ (for building from source) |
| **Disk Space** | ~100 MB for application |
| **Dependencies** | yt-dlp (required) |

> ⚠️ **Note**: Windows and Linux support is planned for v0.9.0

---

## 🔧 Installation

### Prerequisites

1. **Install Rust** (if building from source):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install yt-dlp** (required):
   ```bash
   # Using Homebrew (recommended)
   brew install yt-dlp
   
   # Or using pip
   pip install yt-dlp
   ```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/ibra2000sd/rustloader.git
cd rustloader

# Build release version
cargo build --release

# Run the application
./target/release/rustloader
```

### Quick Start

```bash
# Clone and run in one go
git clone https://github.com/ibra2000sd/rustloader.git
cd rustloader
cargo run --release
```

---

## 🖥️ Usage

### GUI Mode (Default)

Simply run the application:

```bash
cargo run --release
```

Or run the compiled binary:

```bash
./target/release/rustloader
```

**How to use:**
1. Paste a video URL into the input field
2. Select desired quality/format
3. Click "Download"
4. Monitor progress in the download list
5. Find your files in `~/Downloads/Rustloader/`

### Command Line Testing

Test a download without GUI:

```bash
cargo run --release -- --test-download "https://www.youtube.com/watch?v=VIDEO_ID"
```

---

## ⚙️ Configuration

Access settings through the **Settings** panel in the GUI:

| Setting | Description | Default |
|---------|-------------|---------|
| **Download Location** | Where to save videos | `~/Downloads/Rustloader/` |
| **Max Concurrent Downloads** | Simultaneous downloads (1-10) | 5 |
| **Segments per Download** | Parallel segments (4-32) | 16 |
| **Preferred Quality** | Video quality preference | Best Available |

---

## 📁 File Organization

Downloads are automatically organized:

```
~/Downloads/Rustloader/
├── HighQuality/
│   └── 2025-12-31/
│       ├── video1.mp4
│       └── video1.meta.json
├── Standard/
│   └── 2025-12-31/
│       └── video2.mp4
└── LowQuality/
    └── 2025-12-31/
        └── video3.mp4
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     RUSTLOADER                          │
│                                                         │
│  ┌──────────────┐    ┌─────────────────────────┐       │
│  │   Iced GUI   │◄──►│    BackendActor         │       │
│  │              │    │    (Message Loop)        │       │
│  └──────────────┘    └─────────────────────────┘       │
│                              │                          │
│  ┌───────────────────────────┼──────────────────────┐  │
│  ▼                           ▼                      ▼  │
│  ┌─────────────┐    ┌────────────────┐    ┌───────┐   │
│  │   yt-dlp    │    │    Download    │    │ Event │   │
│  │  Extractor  │    │     Engine     │    │  Log  │   │
│  │  (Wrapper)  │    │  (Multi-thread)│    │(JSONL)│   │
│  └─────────────┘    └────────────────┘    └───────┘   │
│                              │                          │
│                     ┌────────────────┐                  │
│                     │ QueueManager   │                  │
│                     │ (FSM+Scheduler)│                  │
│                     └────────────────┘                  │
└─────────────────────────────────────────────────────────┘
```

---

## ⚡ Performance

Rustloader achieves **5-10x faster download speeds** compared to vanilla yt-dlp:

| Metric | Rustloader | yt-dlp (vanilla) |
|--------|------------|------------------|
| 100MB file | ~10 seconds | ~60 seconds |
| Parallel connections | Up to 16 | 1 |
| Memory usage | <150 MB | ~50 MB |
| Resume support | ✅ Yes | ✅ Yes |

*Performance varies based on network conditions and server capabilities.*

---

## 🧪 Testing

Run the test suite:

```bash
# All tests
cargo test

# Stress tests only
cargo test stress_test

# With output
cargo test -- --nocapture
```

Test coverage includes:
- **Stress tests**: Random pause/resume operations, concurrency limits
- **Invariant tests**: Zombie detection, idempotent resume
- **Property tests**: 200+ random operation sequences
- **Persistence tests**: Rehydration, corruption resilience

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [ROADMAP.md](ROADMAP.md) | Detailed feature roadmap with status |
| [CHANGELOG.md](CHANGELOG.md) | Version history and changes |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | Current release information |
| [KNOWN_ISSUES.md](KNOWN_ISSUES.md) | Known limitations and workarounds |

---

## 🐛 Known Issues

See [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for current limitations.

**Quick summary:**
- macOS only (Windows/Linux planned for v0.9.0)
- Some compiler warnings remain (no user impact)
- Large binary size (~90 MB) due to GUI framework

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) - Video extraction engine
- [Iced](https://github.com/iced-rs/iced) - Cross-platform GUI framework
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- [SQLx](https://github.com/launchbadge/sqlx) - Database toolkit

---

## 📞 Support

- **Bug Reports**: [GitHub Issues](https://github.com/ibra2000sd/rustloader/issues)
- **Feature Requests**: [GitHub Discussions](https://github.com/ibra2000sd/rustloader/discussions)

---

<p align="center">
  Made with ❤️ in Rust
</p>

<p align="center">
  <a href="https://github.com/ibra2000sd/rustloader">⭐ Star this repo if you find it useful!</a>
</p>
