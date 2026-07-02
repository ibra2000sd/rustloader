# Known Issues - Rustloader v0.9.0

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

#### ISSUE-007: No disk-space pre-check before starting a download
**Status**: Known limitation
**Impact**: A download larger than the free space on the target volume fails
mid-transfer with a write error instead of being rejected up front; a large
HLS/stream download (whose final size isn't known in advance) can fill the
disk.
**Description**: Neither the native engine nor the yt-dlp path checks
available disk space before or during a download. For direct downloads the
total size is usually known from the server's headers, so a pre-check is
feasible; for HLS/DASH streams the final size is genuinely unknown ahead of
time.
**Workaround**: Make sure the download volume has enough free space,
especially for long streams.
**Target Fix**: Not currently planned.

#### ISSUE-008: Resume progress can be under-reported right after a restart
**Status**: Known limitation (cosmetic)
**Impact**: After resuming an interrupted segmented download, the progress
bar can briefly show less progress than is actually on disk.
**Description**: A resumed segment counts its already-written bytes
(`download_segment_attempt` starts its counter at `existing_bytes`), but
those bytes only show up in the aggregate once that segment has started and
sent its first progress update — segments still waiting to start contribute
nothing yet. Cosmetic only; the resume itself is byte-correct (see ISSUE-001
for scope and the sidecar identity guard).
**Workaround**: None needed; the display converges as segments start
reporting.
**Target Fix**: Not currently planned.

### 🟢 Low Priority

#### ISSUE-004: Optional aria2c downloader is experimental (CLI-only, progress gap)
**Status**: Landed, opt-in via CLI flag
**Impact**: None by default — the feature does nothing unless explicitly
enabled with `--experimental-aria2c`; there is no GUI setting for it.
**Description**: The yt-dlp download path can delegate to an external
`aria2c` (`--downloader aria2c`) via `YtDlpOptions::use_aria2c` (default
`false`), exposed since v0.9.0 as the `--experimental-aria2c` CLI flag
(PR #33). Caveats when enabled: aria2c doesn't support HLS/DASH at all
(yt-dlp silently keeps using its own native downloader for those), so it
only ever applies to plain http/https/ftp transfers routed through yt-dlp —
and for those, yt-dlp only reports progress once the transfer completes, so
the progress bar appears frozen at 0% until it jumps to 100%. Requires
`aria2c` to be installed separately (never bundled).
**Workaround**: N/A — opt-in and clearly labelled experimental.
**Target Fix**: Undecided; needs a GUI toggle and, ideally, a fix for the
progress gap before wider exposure.

#### ISSUE-009: Release binaries are unsigned
**Status**: Known limitation
**Impact**: First launch on macOS and Windows shows a security warning.
**Description**: Release binaries are not code-signed or notarized. macOS
Gatekeeper reports an app from an "unidentified developer" and Windows
SmartScreen shows a "Windows protected your PC" warning. See the README's
"First run on macOS / Windows" section for the standard steps
(right-click → Open or clearing the quarantine attribute on macOS;
"More info" → "Run anyway" on Windows).
**Workaround**: Follow the README first-run steps, and verify the download
against the published `SHA256SUMS.txt` before running it.
**Target Fix**: Not currently planned (signing requires paid developer
certificates).

#### ISSUE-006: Unmaintained/unsound transitive dependencies
**Status**: Monitoring
**Impact**: None currently — no known exploitable vulnerability, only
maintenance-status and soundness advisories from `cargo audit`.
**Description**: Five transitive dependencies pulled in by the Iced GUI
framework are currently flagged as **allowed warnings** (7 warnings total):
`instant` and `paste` (unmaintained), `ttf-parser` (unmaintained, flagged on
three dependency paths), and `lru` and `memmap2` (unsound advisories in code
paths this project doesn't exercise the way the advisory describes). In
addition, `.cargo/audit.toml` carries **justified, documented ignores** for
three advisories: RUSTSEC-2023-0071 (`rsa` via sqlx's mysql metadata — the
mysql feature is disabled and the crate isn't in the built dependency graph)
and RUSTSEC-2026-0194 / RUSTSEC-2026-0195 (quick-xml DoS advisories — its
only path into this project is `wayland-scanner`, a Linux-only build-time
codegen crate parsing trusted vendored XML; quick-xml never runs in the
shipped binary; see PR #40).
**Note**: These will be resolved when Iced (and its own dependencies) update;
tracked via `cargo audit` in CI, which passes (exit 0) with the 7 allowed
warnings and the documented ignores. Revisit the quick-xml ignores when
iced/winit are upgraded.

---

## Resolved in v0.9.0

#### ISSUE-002: Windows/Linux officially supported as of v0.9.0
**Status**: Resolved in v0.9.0
**Impact**: None — all three platforms now have official downloadable builds.
**Description**: v0.9.0 is the first release that officially supports macOS
(arm64 + x86_64), Windows x86_64, and Linux x86_64. CI (`ci.yml`) builds and
runs the full test suite on ubuntu-latest, macos-latest, and windows-latest
for every change, and the release workflow (`release.yml`) publishes
checksummed binaries for all four targets. Note that macOS remains the
platform with the most manual QA history; Windows/Linux are CI-validated on
every change but have a shorter track record of hands-on use — please report
platform-specific issues.
**Workaround**: N/A.

#### ISSUE-003: Orphaned `.partN` files after cancelling a segmented download
**Status**: Resolved in v0.9.0 (PR #36)
**Impact**: None — cancelled/removed downloads no longer leave partial files.
**Description**: `QueueManager::cancel_task` / `remove_task` now best-effort
delete the task's `.partN` files and its `.rustloader-resume` sidecar via
`cleanup_task_artifacts`. Pausing deliberately keeps them — parts and the
sidecar must survive a pause for cross-session resume (ISSUE-001's segmented
path) to work.
**Workaround**: N/A.

#### ISSUE-005: Binary size
**Status**: Resolved (documentation was wrong)
**Impact**: None.
**Description**: Earlier docs claimed a ~90 MB release binary. The actual
released binary is ~6.9 MB (the v0.8.1 macOS arm64 binary measures
7,204,408 bytes; compressed release archives are ~4–6 MB per platform).
**Workaround**: N/A.

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
   - Rustloader version (currently v0.9.0)
   - Any error messages or logs

---

**Last Updated**: 2026-07-02
**Document Version**: 2.1
