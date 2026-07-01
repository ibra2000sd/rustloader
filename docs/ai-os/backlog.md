# Backlog

IDs: `B-<AREA>-NNN` bug, `F-<AREA>-NNN` feature. Status: `open` / `in-progress` /
`closed`. Close items in the same PR that does the work, recording the merge SHA.

Most of the open items below were surfaced by the **aria2 adoption spike (audit,
2026-06-30)** — a read-only investigation whose findings are the source for the
download-reliability work.

## P1 — do first

### B-DL-001 — Segment resume must require HTTP 206, else restart · in-progress · SMALL-MEDIUM
Follow-up to F-DL-002 / PR #28: the resume branch in `download_segment_attempt`
(`segment.rs`) checked only `response.status().is_success()` before appending
to the existing `.partN` file, which accepts a `200 OK` as well as `206`. A
server/CDN/proxy that ignores the `Range` header and returns `200` with the
full body (cache miss, range-coalescing proxy, etc.) would get its body
appended onto the already-written bytes, silently producing an oversized,
corrupt part file with no error raised. **Fix:** on resume
(`existing_bytes > 0`), require `reqwest::StatusCode::PARTIAL_CONTENT`
(optionally cross-checked against the `Content-Range` start offset when
present) before appending; any other status truncates the stale partial and
returns `Err` so the retry loop restarts the segment fresh. The first-attempt
(`existing_bytes == 0`) path is unchanged. Regression test added:
`test_resume_restarts_when_server_ignores_range`. PR opened (not yet merged);
close this item with the merge SHA once it lands.

### F-DL-002 — Segment-failure tolerance: don't abort the whole download · in-progress · MEDIUM
When any single segment errors, the engine `break`s and fails the **entire**
download (`engine.rs` result loop), and per-segment retries truncate from byte 0
(`segment.rs` `File::create`). A single dropped connection to a throttled/capped
host therefore kills a large transfer. **Fix:** tolerate/retry a failed segment
without aborting the whole download, and make segment retry resume from the
already-written bytes instead of re-downloading from 0. This is the cheaper half
of audit "Shape D" and independently fixes the throttled-host failure mode.
Source: internal audit 2026-06-30.
**Status:** the retry-resume half is done — `segment.rs` retries now resume
from already-written bytes (Range + append, cumulative progress, wall-clock-
bounded retry budget) instead of truncating. PR
[#28](https://github.com/ibra2000sd/rustloader/pull/28) (open, not yet
merged). The engine's `break` on a genuinely-unrecoverable segment is
intentionally retained/unchanged — see the PR description. Close this item
with the merge SHA once #28 lands.

## P2

### F-DL-001 — Shape A: use aria2c as yt-dlp's external downloader · open · SMALL (XS)
Add `--downloader aria2c` (+ `--downloader-args`) to `build_ytdlp_args` when an
**external** `aria2c` is detected (mirror `find_ytdlp` detection; do NOT bundle —
aria2 is GPL-2.0, see `adr/0002`). Improves multi-connection + resume on the
yt-dlp/HLS download path only; no change to the progress contract (yt-dlp still
drives). Gated on B-DOC-001 (license posture) before any aria2 dependency lands.
Source: internal audit 2026-06-30.

### F-DL-003 — Byte-level resume + checkpoint persistence (native engine) · open · MEDIUM-LARGE
Wire the dead `enable_resume` flag into real behavior: stop truncating
(`OpenOptions` append + offset `Range`), persist a per-segment checkpoint, and
turn pause/resume from restart into true byte-resume.
**Investigate first:** the `download_segments` SQLite table is already
written/read in `database/operations.rs` — determine whether resume is largely a
**wire-up** of existing persistence rather than a build from scratch (the audit
characterized progress as in-memory-only and under-explored this DB layer).
This is the larger half of audit "Shape D". Preferred over adopting aria2 for the
native path (no GPL dependency, full progress/pause parity retained).
Source: internal audit 2026-06-30.

### B-DOC-002 — KNOWN_ISSUES.md content is stale · open · SMALL
`B-DOC-001` fixed only the title's version stamp (now "v0.8.1"); the body still
doesn't reflect the throttle/resume limitations. A full content refresh remains
open. Source: internal audit 2026-06-30.

## P3 / later

### F-EXTRACT-001 — Proxy-capture spike (res-downloader style) · open · investigate-first
Exploratory spike for a local-proxy media capture ("any page that plays video",
no extractor). Heavy: requires a system proxy + a local CA certificate in the
trust store (real security/UX surface). **Scope boundary (mandatory):** legitimate
capture only (public content without an extractor, content the user owns, m3u8 the
browser already plays); explicitly **excludes** defeating DRM/content protection.
Reference upstream `putyy/res-downloader` is Apache-2.0 (Go+Wails — port the
approach, not the code). Relates to the CHANGELOG's "Browser extension
integration (v1.0.0)" plan. Decision needed from maintainer before drafting.

### B-DL-003 (optional) — Reconsider the 1800s yt-dlp download timeout · open · SMALL
`download_via_ytdlp` is correctly bounded but 30 min is generous; consider
lowering / making it configurable. NOT a bug (it is already bounded).

## Recently closed

| ID | Title | Closed by |
|----|-------|-----------|
| — | Tolerate float `duration` in `VideoInfo` deserialize | PR #21 (`dff16f2`) |
| — | Robust default yt-dlp format selector (HLS master) | PR #22 (`933b2c0`) |
| — | Bound yt-dlp **extraction** subprocess with timeout + kill | PR #23 (`1c038e2`) |
| `B-DOC-001` | README/LICENSE/roadmap claims are inaccurate | `f897872`, 2026-07-01 (PR pending) |

(Pre-`docs/ai-os` work was tracked via GitHub PRs/CHANGELOG; future items use the
IDs above.)
