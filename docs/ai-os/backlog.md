# Backlog

IDs: `B-<AREA>-NNN` bug, `F-<AREA>-NNN` feature. Status: `open` / `in-progress` /
`closed`. Close items in the same PR that does the work, recording the merge SHA.

Most of the open items below were surfaced by the **aria2 adoption spike (audit,
2026-06-30)** — a read-only investigation whose findings are the source for the
download-reliability work.

## P1 — do first

### B-DL-001 — Segment resume must require HTTP 206, else restart · closed · SMALL-MEDIUM
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
`test_resume_restarts_when_server_ignores_range`. PR
[#29](https://github.com/ibra2000sd/rustloader/pull/29), merged `c976872`
(2026-07-01).

### F-DL-002 — Segment-failure tolerance: don't abort the whole download · closed (retry-resume half) · MEDIUM
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
[#28](https://github.com/ibra2000sd/rustloader/pull/28), merged `c1c0580`
(2026-07-01). The engine's `break` on a genuinely-unrecoverable segment is
intentionally retained/unchanged — see the PR description. The
whole-download-abort-tolerance half (letting the engine survive a segment
that never recovers) remains open, tracked separately if pursued.

## P2

### F-DL-001 — Shape A: use aria2c as yt-dlp's external downloader · open · SMALL (XS)
Add `--downloader aria2c` (+ `--downloader-args`) to `build_ytdlp_args` when an
**external** `aria2c` is detected (mirror `find_ytdlp` detection; do NOT bundle —
aria2 is GPL-2.0, see `adr/0002`). Improves multi-connection + resume on the
yt-dlp/HLS download path only; no change to the progress contract (yt-dlp still
drives). Gated on B-DOC-001 (license posture) before any aria2 dependency lands.
Source: internal audit 2026-06-30.

### F-DL-003 — Byte-level resume + checkpoint persistence (native engine) · in-progress · MEDIUM-LARGE
**Correction (2026-07-01 spike):** the previous framing of this item — that the
`download_segments` SQLite table is "already written/read in
`database/operations.rs`" and that resume is therefore largely a DB wire-up —
was stale/inaccurate. A read-only spike verified `save_segment`/`get_segments`/
`save_download`/`get_download`/`get_all_downloads`/`get_downloads_by_status`/
`delete_download` have **zero callers anywhere outside `database/operations.rs`
itself**; the table is fully dead, not partially wired. The spike also found
that #28/#29 already made cross-session resume *happen* as an unintentional
side effect (deterministic `calculate_segments`, no `.partN` cleanup on pause/
cancel/app-close, `output_path`/URL preserved via the in-memory task or the
`EventLog`) — but with **zero validation** that the on-disk parts belong to the
current plan, which is a latent silent-corruption bug, not a missing feature.
See the spike report (session transcript, 2026-07-01) for full evidence.

**Fix landed (Shape 2 — sidecar identity guard, PR
[#30](https://github.com/ibra2000sd/rustloader/pull/30), open, not yet
merged):** a small `<output>.rustloader-resume` sidecar records
`{url_hash, file_size, segment_count}` before segment downloads start. On
every `download()` call, an existing `.partN` set is only trusted if the
sidecar matches the current identity *and* `enable_resume` is `true`;
otherwise (mismatch, missing sidecar with parts present, or resume disabled)
the parts are discarded and the segment loop starts clean. This closes the
two corruption paths: a segment-count preference change between sessions, and
a different download reusing the same `output_path`. `enable_resume` finally
gates real behavior instead of being a dead, always-on flag. Close this item
with the merge SHA once #30 lands.

**Deliberately NOT done here (spinoffs, separate items):**
- **Orphaned `.partN` cleanup on cancel/remove** — `pause_task`/`cancel_task`/
  `remove_task` (`queue/manager.rs`) never call `cleanup_segments`, so
  cancelled downloads leave parts on disk indefinitely. The identity guard
  makes this *safe* (a mismatch/foreign check would clean them up on the next
  attempt at that path), but the litter itself is unaddressed. File as its own
  small item if wanted.
- **Shape 3 / DB-backed persistence** — using `downloads`/`download_segments`
  to store the plan instead of (or in addition to) the filesystem sidecar
  remains a legitimate future direction (would also unlock download history/
  resume across a moved output path), but is out of scope for this fix.
Source: internal audit 2026-06-30; F-DL-003 spike 2026-07-01.

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
| `F-DL-002` | Segment retry resumes from written bytes (retry-resume half) | PR #28, `c1c0580`, 2026-07-01 |
| `B-DL-001` | Segment resume requires HTTP 206, else restarts | PR #29, `c976872`, 2026-07-01 |

(Pre-`docs/ai-os` work was tracked via GitHub PRs/CHANGELOG; future items use the
IDs above.)
