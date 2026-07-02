# Rustloader v0.9.0 Release Notes

**Release Date**: July 2026
**Type**: Feature & Reliability Release
**Status**: Beta

---

## Overview

Rustloader v0.9.0 is the first release to **officially support all three
platforms** — macOS (arm64 + x86_64), Windows x86_64, and Linux x86_64 — with
pre-built, checksummed binaries for each. It also ships real byte-level
resume for segmented downloads, a persistent download history with a GUI
view, and a batch of GUI and download-engine reliability fixes.

For the full commit-level list, see the [CHANGELOG](CHANGELOG.md).

---

## What's New

### 🖥️ Official Windows & Linux Support

Every change is built and tested by CI on ubuntu-latest, macos-latest, and
windows-latest, and the release workflow publishes binaries plus a
`SHA256SUMS.txt` for all four targets. macOS remains the platform with the
longest manual-QA history — please report platform-specific issues on the
tracker.

### ⏯️ Byte-Level Resume (segmented downloads)

- Interrupted segment transfers resume from the bytes already written
  instead of restarting from zero (#28).
- A resume is only trusted when the server honors `Range` with
  `206 Partial Content`; otherwise the segment restarts clean instead of
  silently corrupting the file (#29).
- A `.rustloader-resume` identity sidecar (URL + file size + segment count)
  guards cross-session resume, so stale or foreign partial files are
  discarded, never merged (#30).

Scope: this covers direct media downloads ≥1MB on `Range`-capable servers;
small files and the yt-dlp/HLS path still restart on interruption (see
[KNOWN_ISSUES.md](KNOWN_ISSUES.md), ISSUE-001).

### 📜 Download History

Downloads are persisted to the database across their full lifecycle (#34)
and browsable in a new History sidebar view with Remove-from-history and
Show-in-Folder actions (#35).

### 🔧 Reliability Fixes

- GUI: downloads actually start; settings-save errors are surfaced (#19, #20)
- Extraction: tolerant yt-dlp JSON deserialization (#21), robust HLS-master
  format selector (#22), and a 60s timeout + kill bound on the yt-dlp
  subprocess (#23)
- Cancelling/removing a download cleans up its `.partN` files and resume
  sidecar (#36)
- The engine's blanket 30s total request timeout is replaced by
  connect + stall timeouts, so slow-but-progressing transfers complete (#37)
- Saved file extensions are derived from the actual content (#39)

### 🧪 Experimental

- `--experimental-aria2c` CLI flag lets yt-dlp delegate plain
  http/https/ftp transfers to an external `aria2c` (#31, #33). Off by
  default; see [KNOWN_ISSUES.md](KNOWN_ISSUES.md) ISSUE-004 for caveats.

---

## Installation

### Requirements

- macOS 10.15+, Windows 10+, or Linux x86_64 (Ubuntu 22.04+, Fedora 38+)
- `yt-dlp` installed and on PATH
- `ffmpeg` (mp3 extraction and yt-dlp postprocessing)
- A JavaScript runtime (`deno` or `node`) — needed by modern yt-dlp for
  YouTube

### Install

Download the binary for your platform from
[GitHub Releases](https://github.com/ibra2000sd/rustloader/releases) and
verify it against `SHA256SUMS.txt`. Binaries are **unsigned** — see the
README's "First run on macOS / Windows" section for the Gatekeeper /
SmartScreen steps.

---

## Known Limitations

See [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for the full list, including resume
scope (ISSUE-001), the missing disk-space pre-check (ISSUE-007), and the
unsigned-binaries first-run warnings (ISSUE-009).

---

## Feedback & Support

- **Bug Reports**: [GitHub Issues](https://github.com/ibra2000sd/rustloader/issues)
- **Feature Requests**: [GitHub Discussions](https://github.com/ibra2000sd/rustloader/discussions)
