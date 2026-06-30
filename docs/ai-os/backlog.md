# Backlog

IDs: `B-<AREA>-NNN` bug, `F-<AREA>-NNN` feature. Status: `open` / `in-progress` /
`closed`. Close items in the same PR that does the work, recording the merge SHA.

Most of the open items below were surfaced by the **aria2 adoption spike (audit,
2026-06-30)** — a read-only investigation whose findings are the source for the
download-reliability work.

## P1 — do first

### F-DL-002 — Segment-failure tolerance: don't abort the whole download · open · MEDIUM
When any single segment errors, the engine `break`s and fails the **entire**
download (`engine.rs` result loop), and per-segment retries truncate from byte 0
(`segment.rs` `File::create`). A single dropped connection to a throttled/capped
host therefore kills a large transfer. **Fix:** tolerate/retry a failed segment
without aborting the whole download, and make segment retry resume from the
already-written bytes instead of re-downloading from 0. This is the cheaper half
of audit "Shape D" and independently fixes the throttled-host failure mode.
Source: internal audit 2026-06-30.

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

(Pre-`docs/ai-os` work was tracked via GitHub PRs/CHANGELOG; future items use the
IDs above.)
