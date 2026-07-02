# 🚀 Rustloader - High-Performance Video Downloader

[![Version](https://img.shields.io/badge/version-0.9.0-blue.svg)](https://github.com/ibra2000sd/rustloader/releases)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/ibra2000sd/rustloader)
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
| **v0.8.x** | ✅ Complete | First published release; byte-level segmented resume, download reliability, cookies support |
| **v0.9.x** | ✅ Complete | Official Windows & Linux support, download history, orphan cleanup, engine timeout fixes |
| **v1.0.0** | 🔴 Planned | Production release with full test coverage |

See [ROADMAP.md](ROADMAP.md) for detailed feature breakdown and implementation status.

---

## ✨ What's New

### v0.9.0 Highlights

- 🖥️ **Official Windows & Linux support**: pre-built, checksummed binaries for macOS (arm64 + x86_64), Windows x86_64, and Linux x86_64
- ⏯️ **Byte-level resume**: interrupted segmented downloads resume from the bytes already on disk, guarded by an identity sidecar so stale/foreign partials are never merged
- 📜 **Download history**: downloads persist to the database and are browsable in a new History view
- 🧹 **Orphan cleanup**: cancelling/removing a download removes its `.partN` files and resume sidecar
- ⏱️ **Smarter timeouts**: slow-but-progressing transfers complete; dead connections still abort within a bounded window
- 🏷️ **Correct file extensions**: the saved extension is derived from the actual content, not the requested mode

See the [CHANGELOG](CHANGELOG.md) for the full list.

---

## 🎯 Features

| Feature | Description |
|---------|-------------|
| **Multi-threaded Downloads** | Up to 16 parallel segments for maximum speed |
| **1000+ Site Support** | Powered by yt-dlp for broad compatibility |
| **Pause/Resume Control** | Byte-level resume for segmented direct downloads (identity-guarded, works across app restarts); other paths restart the transfer — see [KNOWN_ISSUES.md](KNOWN_ISSUES.md#issue-001-resume-scope-is-limited-to-segmented-direct-media-downloads) |
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
| **Operating System** | macOS 10.15+, Windows 10+, or Linux x86_64 (Ubuntu 22.04+, Fedora 38+) |
| **Rust** | 1.70+ (for building from source) |
| **Disk Space** | ~10 MB for the application (~6.9 MB binary) |
| **Dependencies** | yt-dlp (required); ffmpeg (required for mp3 extraction and yt-dlp postprocessing); a JavaScript runtime — deno or node — (needed by modern yt-dlp for YouTube) |

---

## 🔧 Installation

### Download from Releases (recommended)

Pre-built binaries for every supported platform are published on
[GitHub Releases](https://github.com/ibra2000sd/rustloader/releases), with a
`SHA256SUMS.txt` for verification.

**macOS (Apple Silicon)**
```bash
curl -LO https://github.com/ibra2000sd/rustloader/releases/latest/download/rustloader-macos-arm64.tar.gz
tar xzf rustloader-macos-arm64.tar.gz
./rustloader
```

**macOS (Intel)**
```bash
curl -LO https://github.com/ibra2000sd/rustloader/releases/latest/download/rustloader-macos-x86_64.tar.gz
tar xzf rustloader-macos-x86_64.tar.gz
./rustloader
```

**Windows (x64)** — download
`rustloader-windows-x86_64.zip` from the
[Releases page](https://github.com/ibra2000sd/rustloader/releases), extract
it, and run `rustloader.exe`.

**Linux (x64)**
```bash
curl -LO https://github.com/ibra2000sd/rustloader/releases/latest/download/rustloader-linux-x86_64.tar.gz
tar xzf rustloader-linux-x86_64.tar.gz
./rustloader
```

To verify a download, compare `shasum -a 256 <file>` (or `sha256sum` on
Linux, `certutil -hashfile <file> SHA256` on Windows) against
`SHA256SUMS.txt` from the same release.

### First run on macOS / Windows (unsigned binaries)

Release binaries are **not code-signed**, so the OS will warn on first launch:

- **macOS (Gatekeeper)**: "cannot be opened because it is from an
  unidentified developer." Either right-click the binary → **Open** → **Open**
  (once is enough), or clear the quarantine attribute:
  ```bash
  xattr -d com.apple.quarantine ./rustloader
  ```
- **Windows (SmartScreen)**: "Windows protected your PC." Click
  **More info** → **Run anyway**.

Verifying the SHA256 checksum first (above) is recommended.

### Prerequisites

1. **Install yt-dlp** (required):
   ```bash
   # macOS (Homebrew)
   brew install yt-dlp

   # Or using pip (any platform)
   pip install yt-dlp
   ```

2. **Install ffmpeg** (required for mp3 extraction and yt-dlp
   postprocessing/merging):
   ```bash
   # macOS
   brew install ffmpeg

   # Debian/Ubuntu
   sudo apt install ffmpeg
   ```

3. **Install a JavaScript runtime** — modern yt-dlp needs `deno` or `node`
   on your PATH to solve YouTube's JS challenges (Rustloader's startup
   dependency check warns if one is missing):
   ```bash
   curl -fsSL https://deno.land/install.sh | sh
   ```

4. **Install Rust** (only if building from source):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
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

### Command Line Mode

Pass a URL to download without the GUI (see `rustloader --help` for quality,
format, clip, subtitle, playlist, and cookie flags):

```bash
cargo run --release -- "https://www.youtube.com/watch?v=VIDEO_ID"
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

Rustloader splits a download into up to 16 parallel HTTP range requests. The
speedup this yields is **conditional on the link** — so we measured it rather
than guessing:

| Scenario | Single stream | Rustloader (16 segments) | Speedup |
|----------|---------------|--------------------------|---------|
| Per-connection-throttled link (server caps each connection; spare aggregate bandwidth) | 57.6 s | 11.3 s | **~5×** |
| Unthrottled / total-bandwidth-capped link | baseline | ≈ baseline | **~1×** |

<sub>Measured on a 150 MB file, median of 3 alternating trials (single-stream `curl` vs the 16-segment engine), outputs verified byte-identical via SHA-256. The ~5× row used a 3 MiB/s per-connection cap with spare aggregate bandwidth; the ~1× row reflects links where the bottleneck is total bandwidth, not per-connection limits.</sub>

**When the extra connections actually help:** multi-segment downloading only
beats a single stream when each connection is individually rate-limited (common
on some file hosts) or on high-latency links. When your *total* bandwidth is the
bottleneck — typical home/office connections and unthrottled CDNs — all 16
segments share the same pipe, so the result is ≈1×. On links that drop idle
connections, splitting one slow stream into 16 slower ones can even be *less*
reliable than a single stream, so the engine falls back to a single stream for
small files and servers that don't support range requests.

| Metric | Rustloader | yt-dlp (vanilla) |
|--------|------------|------------------|
| Parallel connections | Up to 16 | 1 |
| Resume support | ✅ Byte-level for segmented direct downloads (identity-guarded, survives app restarts); restart-only for small files and the yt-dlp/HLS path | ✅ Yes |

*Performance varies with network conditions and server behavior.*

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
- Resume is byte-level only for segmented direct downloads; small files and the yt-dlp/HLS path restart on interruption
- No disk-space pre-check before starting a download
- Release binaries are unsigned (see "First run on macOS / Windows" above)

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
