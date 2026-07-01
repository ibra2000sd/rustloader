# Known Issues - Rustloader v0.8.1

This document tracks known issues and limitations in the current release.

---

## Current Issues

### 🟡 Medium Priority

#### ISSUE-001: Resume scope is limited to segmented direct-media downloads
**Status**: Partially addressed
**Impact**: Some downloads restart from zero on interruption instead of resuming.
**Description**: Byte-level resume (`downloader::resume_guard`) covers the
native engine's **segmented** path — direct media files ≥1MB from a server
that supports HTTP `Range` requests. A `.partN` sidecar records the download's
identity (URL + file size + segment count); on the next attempt (whether
that's a pause/resume in the same run or a fresh run after the app was
closed), existing parts are only trusted if the sidecar matches, otherwise
the download restarts clean rather than risking a corrupt file. Two cases are
**not** covered:
- **Small files (<1MB) or hosts that don't support `Range`** use the engine's
  simple download path, which always fetches from scratch — no resume at all.
- **The yt-dlp/HLS/DASH fallback path** (`download_via_ytdlp`, used for
  streaming-site and complex sources) has no resume logic of its own; any
  continuation behavior on retry comes from yt-dlp itself, not Rustloader.
**Workaround**: None needed for the segmented case — it just works. For the
other two cases, an interrupted download needs to be restarted manually.
**Target Fix**: Not currently planned; the segmented case covers the failure
mode (throttled/dropped connections on large direct-media transfers) the
download-reliability work targeted.

#### ISSUE-002: Windows/Linux are CI-validated but not yet officially released
**Status**: In progress
**Impact**: Users on Windows/Linux have no official downloadable build yet.
**Description**: CI (`ci.yml`) builds and runs the full test suite on
ubuntu-latest, macos-latest, and windows-latest for every change, and the
release workflow (`release.yml`) is already configured to produce
Windows/Linux/macOS artifacts. However, macOS is still the only platform
that's been through manual QA and is distributed as a supported release —
Windows/Linux support remains the ROADMAP's next milestone (v0.9.0), not yet
promoted to a released, supported target.
**Workaround**: Build from source with `cargo build --release` on your
platform; expect rough edges (untested manually) outside macOS.
**Target Fix**: v0.9.0

#### ISSUE-003: Orphaned `.partN` files after cancelling a segmented download
**Status**: Known limitation
**Impact**: Cancelled downloads can leave partial `.partN` files in the
output directory, taking up disk space.
**Description**: Segment part files are cleaned up automatically after a
successful merge, but cancelling or removing an in-progress task
(`QueueManager::cancel_task` / `remove_task`) doesn't clean them up. This is
disk hygiene, not a correctness risk: the resume-identity guard (ISSUE-001)
means a later download can never silently pick up a stale/foreign `.partN`
by mistake — worst case it's discarded and refetched.
**Workaround**: Manually delete `.partN` files (and any
`.rustloader-resume` sidecar) left in the download directory after
cancelling.
**Target Fix**: Not currently planned.

### 🟢 Low Priority

#### ISSUE-004: Optional aria2c downloader is experimental and not yet exposed
**Status**: Landed, opt-in only
**Impact**: None by default — the feature does nothing unless explicitly
enabled programmatically; there's no CLI flag or GUI setting for it yet.
**Description**: The yt-dlp download path can delegate to an external
`aria2c` (`--downloader aria2c`) via `YtDlpOptions::use_aria2c`, which
defaults to `false` and isn't currently set to `true` anywhere in the CLI or
GUI. Even when enabled by a future integration, be aware: aria2c doesn't
support HLS/DASH at all (yt-dlp silently keeps using its own native
downloader for those), so it only ever applies to plain http/https/ftp
transfers routed through yt-dlp — and for those, yt-dlp only reports
progress once the transfer completes, so the progress bar would appear
frozen at 0% until it jumps to 100%.
**Workaround**: N/A — not user-reachable in this release.
**Target Fix**: Undecided; needs a CLI/GUI toggle and, ideally, a fix for
the progress gap before it's worth exposing.

#### ISSUE-005: Large Binary Size
**Status**: Accepted
**Impact**: Minor (longer download time)
**Description**: Release binary is ~90 MB due to GUI framework dependencies.
**Workaround**: None - this is expected for Iced-based applications.

#### ISSUE-006: Unmaintained/unsound transitive dependencies
**Status**: Monitoring
**Impact**: None currently — no known exploitable vulnerability, only
maintenance-status and soundness advisories from `cargo audit`.
**Description**: Five transitive dependencies pulled in by the Iced GUI
framework are currently flagged: `instant` and `paste` (unmaintained),
`ttf-parser` (unmaintained), and `lru` and `memmap2` (unsound advisories in
code paths this project doesn't exercise the way the advisory describes).
**Note**: These will be resolved when Iced (and its own dependencies) update;
tracked via `cargo audit` in CI, which currently passes with these as
allowed warnings, not blocking failures.

---

## Recently Resolved (v0.8.x download-reliability work)

| Item | Description | Resolved by |
|------|-------------|-------------|
| `F-DL-002` | Segment retries resume from already-written bytes instead of restarting from byte 0 on a dropped/throttled connection | PR #28 (`c1c0580`) |
| `B-DL-001` | Segment resume now requires the server to honor `206 Partial Content`; a server that silently ignores `Range` no longer corrupts the output | PR #29 (`c976872`) |
| `F-DL-003` | Cross-session resume (pause, or closing and reopening the app) is now identity-guarded via a resume sidecar, closing a silent-corruption gap | PR #30 (`f51dfad`) |

## Resolved Issues (v0.1.1, historical)

| Issue | Description | Resolution |
|-------|-------------|------------|
| BUG-001 | Mutex deadlock in video extraction | Fixed - lock released before await |
| BUG-004 | Pause/Resume/Cancel non-functional | Fixed - proper state management |
| BUG-006 | Progress bars empty for subsequent downloads | Fixed - improved tracking |
| BUG-007 | Files not organized into directories | Fixed - directory validation |
| BUG-008 | Pause buttons disappear | Fixed - state string handling |
| SEC-001 | Path traversal in filenames | Fixed - comprehensive sanitization |

---

## Reporting New Issues

If you encounter a bug not listed here:

1. **Search existing issues**: [GitHub Issues](https://github.com/ibra2000sd/rustloader/issues)
2. **Create a new issue** with:
   - Steps to reproduce
   - Expected vs actual behavior
   - Your OS and version
   - Rustloader version (currently v0.8.1)
   - Any error messages or logs

---

**Last Updated**: July 2026
**Document Version**: 2.0
