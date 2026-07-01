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

### F-DL-001 — Shape A: use aria2c as yt-dlp's external downloader · closed (opt-in) · SMALL (XS)
Add `--downloader aria2c` to `build_ytdlp_args` when an **external** `aria2c` is
detected (mirror `find_ytdlp` detection; do NOT bundle — aria2 is GPL-2.0, see
`adr/0002`). Gated on B-DOC-001 (license posture); landed after it.
Source: internal audit 2026-06-30.

**Correction (2026-07-01 implementation) — the progress-contract assumption
above was wrong, verified empirically, not assumed:** live-smoke-tested
`yt-dlp --downloader aria2c` against both an HLS stream and a direct HTTP file,
and read yt-dlp's own `downloader/external.py` source. Two findings:
1. **aria2c never engages for HLS/DASH at all** —
   `Aria2cFD.SUPPORTED_PROTOCOLS = ('http', 'https', 'ftp', 'ftps')` in yt-dlp's
   source; `m3u8`/`dash` aren't in that list, so yt-dlp silently falls back to
   its native `hlsnative` downloader regardless of the flag. Confirmed live: a
   `--downloader aria2c` run against an HLS test stream produced
   byte-identical `[hlsnative]`-tagged log output to a run without the flag —
   i.e. **zero benefit for `download_via_ytdlp`'s primary use case** (HLS/DASH
   fallback).
2. **Where aria2c *does* engage (plain http/https/ftp), progress breaks.**
   yt-dlp's `ExternalFD.real_download()` calls its progress hook exactly
   **once, on completion** — not incrementally during the transfer. Live-smoke
   confirmed: a direct-HTTP-file run with `--downloader aria2c` printed
   aria2c's own raw progress format (`[#10ff12 ...]`) throughout the transfer,
   and only the very last line matched `parse_yt_dlp_progress`'s expected
   `[download] X% of Y at Z` shape (the 100%-at-completion line). The
   intermediate aria2c-format lines partially misparse (a coincidental digits-
   before-`%` match extracts a plausible-looking percentage, but `total_bytes`
   parses to `0`, which `DownloadProgress::percentage()` clamps to `0%`) — net
   effect: the progress bar sits frozen at 0% for the whole transfer, then
   jumps to 100%. This is a real I-3 regression for the one case the flag
   would actually change anything.

**Fix landed (PR [#31](https://github.com/ibra2000sd/rustloader/pull/31),
open, not yet merged):** implemented as **opt-in, default off**
(`YtDlpOptions::use_aria2c: bool`, defaults `false` via `derive(Default)`).
`find_aria2c()` (`extractor/ytdlp.rs`, mirrors `find_ytdlp`/
`find_in_common_paths`, deliberately **without** `find_ytdlp`'s
bundled/adjacent-to-executable check — I-9/ADR-0002) detects an external
aria2c; `build_ytdlp_args` takes the caller's already-resolved
`aria2c_available: bool` and only then adds `--downloader aria2c`. No CLI/GUI
toggle is wired up in this PR (nothing currently sets `use_aria2c: true`), so
this PR changes nothing by default — verified via `cargo run -- <url>
--dry-run`, byte-identical args to before. Exposing it as a real opt-in
(CLI flag or GUI setting) is a follow-up, once/if the progress-hook gap above
is separately addressed (or accepted as a documented tradeoff for that
narrow case).

**F-DL-001b — enable path (PR
[#33](https://github.com/ibra2000sd/rustloader/pull/33), open, not yet
merged):** added the actual opt-in: `Cli::experimental_aria2c` (CLI flag
`--experimental-aria2c`, default `false`), threaded to
`YtDlpOptions::use_aria2c` in `to_ytdlp_options()`. Help text labels it
`EXPERIMENTAL` up front and states the progress-freeze caveat from the
correction above verbatim, so turning it on is a conscious, informed choice,
not a casual toggle. No GUI checkbox added — `gui/app.rs` has no existing
settings-UI widgets (checkboxes/toggles) to attach a "advanced/experimental"
option to today, and the task explicitly discouraged a prominent toggle;
adding one properly (its own advanced-settings area, non-prominent, labelled)
would be a materially larger change than "wire the CLI flag." Left as an
explicit follow-up. Absent the flag, `build_ytdlp_args`'s output is verified
byte-identical to before (`cargo run -- <url> --dry-run`, no `--downloader`
in the args); with the flag and a real `aria2c` installed, the same command
shows `--downloader aria2c`.

### F-DL-003 — Byte-level resume + checkpoint persistence (native engine) · closed (Shape 2) · MEDIUM-LARGE
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
[#30](https://github.com/ibra2000sd/rustloader/pull/30), merged `f51dfad`,
2026-07-01):** a small `<output>.rustloader-resume` sidecar records
`{url_hash, file_size, segment_count}` before segment downloads start. On
every `download()` call, an existing `.partN` set is only trusted if the
sidecar matches the current identity *and* `enable_resume` is `true`;
otherwise (mismatch, missing sidecar with parts present, or resume disabled)
the parts are discarded and the segment loop starts clean. This closes the
two corruption paths: a segment-count preference change between sessions, and
a different download reusing the same `output_path`. `enable_resume` finally
gates real behavior instead of being a dead, always-on flag.

**Deliberately NOT done here (spinoffs, separate items):**
- **Orphaned `.partN` cleanup on cancel/remove** — `pause_task`/`cancel_task`/
  `remove_task` (`queue/manager.rs`) never call `cleanup_segments`, so
  cancelled downloads leave parts on disk indefinitely. The identity guard
  makes this *safe* (a mismatch/foreign check would clean them up on the next
  attempt at that path), but the litter itself is unaddressed. **Filed and
  done as `B-DL-002` below.**
- **Shape 3 / DB-backed persistence** — using `downloads`/`download_segments`
  to store the plan instead of (or in addition to) the filesystem sidecar
  remains a legitimate future direction (would also unlock download history/
  resume across a moved output path), but is out of scope for this fix. The
  `downloads` half (history, not resume) is now `F-HIST-001` below;
  `download_segments`-backed resume remains unaddressed and out of scope
  there too — the sidecar still owns resume.
Source: internal audit 2026-06-30; F-DL-003 spike 2026-07-01.

### B-DL-002 — Orphaned `.partN` + resume sidecar left behind on cancel/remove · closed (PR open) · SMALL
The F-DL-003 hygiene spinoff (see the "Deliberately NOT done" list above):
`cleanup_segments` ran only after a successful merge (`engine.rs`), so
`cancel_task`/`remove_task` left the `.partN` files and the
`<output>.rustloader-resume` sidecar on disk indefinitely — safe (the sidecar
identity guard prevents corruption from stale parts) but litter. **Fix (PR
[#36](https://github.com/ibra2000sd/rustloader/pull/36), open, not yet
merged):** a private best-effort `cleanup_task_artifacts` helper in
`queue/manager.rs` removes the sidecar (via `resume_guard`'s
`sidecar_path`/`remove_sidecar`) and every `<file_name>.part<digits>` file in
the output's directory (a strict digit-suffix scan — the segment count isn't
known at the queue layer). Called from `cancel_task` and `remove_task` only,
after the locks are released; failures are logged, never propagated.
`pause_task` deliberately does NOT clean up — parts + sidecar must survive
pause or cross-session resume (F-DL-003/#30) breaks; `clear_completed` needs
nothing because the engine already removes both at merge time
(`engine.rs`). Regression tests in `tests/orphan_cleanup_test.rs`: cancel
and remove delete the artifacts (decoys untouched), and — the load-bearing
guard — pause keeps them.
Source: F-DL-003 spinoff, 2026-07-01; implemented 2026-07-02.

### F-HIST-001 — Shape-3 PR-1: persist download history to the `downloads` table · closed (headless) · MEDIUM-LARGE
The `downloads` table (and its CRUD — `save_download`/`get_download`/
`get_all_downloads`/`get_downloads_by_status`/`delete_download`) has been dead
since it was first defined — zero callers anywhere outside
`database/operations.rs` itself (confirmed by the F-DL-003 spike, re-confirmed
at this item's own HEAD). This wires it into the live download lifecycle as a
durable **history** (survives an app restart), separate from — and not a
replacement for — the sidecar-based resume mechanism (#30) or the EventLog's
live-queue rehydrate.

**Design decisions (see PR
[#34](https://github.com/ibra2000sd/rustloader/pull/34) description for full
detail):**
- **Injection:** `gui/app.rs` already builds one `DatabaseManager` (used today
  only for the `settings` table); `BackendActor::new` now takes an
  `Arc<DatabaseManager>` — the SAME instance, cloned via `Arc::clone`, not a
  second pool/file — and stores it. `QueueManager` itself is untouched.
- **Identity:** `downloads.id` = the queue task ID
  (`Uuid::new_v4().to_string()`, generated in `handle_start_download`) — the
  same ID `EventLog`/`DownloadTask.id` already use, so a history row and a
  live queue task reconcile 1:1 by construction.
- **EventLog coherence:** `QueueManager`/`EventLog` remain the sole runtime
  authority for a task's live state (I-4 unchanged — `queue/manager.rs` and
  `queue/events.rs` have zero diff in this PR). The `downloads` table is a
  best-effort, derived, write-only *projection* of that authority: one row is
  inserted (status `Queued`) right after a successful `queue_manager.add_task`
  in `handle_start_download`, and updated (same row, `INSERT OR REPLACE`
  keyed by id) on every subsequent status transition, detected by the
  existing `monitor_loop` polling diff (`backend/actor.rs`) that already
  drove the GUI's `TaskStatusUpdated` event — no new detection mechanism, no
  second lock/authority. Removing a task from the live queue (cancel/remove)
  does **not** delete its history row — history is meant to outlive the live
  queue entry, that's the point of a persistent history.
- **`download_segments` stays dead** — not wired, not touched. Resume is
  still exclusively the `.rustloader-resume` sidecar's job.

**Tests:** `database/operations.rs` gained two regression tests —
`download_history_survives_reopening_the_database` (writes rows, drops the
pool, opens a brand-new pool against the same file, reads them back via
`get_all_downloads` with correct status/fields — the literal "survives a
simulated restart" acceptance bar) and
`status_transitions_update_in_place_not_duplicate` (three transitions of one
task id via `save_download` leave exactly one row, not three, with the final
status/`completed_at` and the original `created_at` preserved). Two pure unit
tests for the new `task_status_db_fields` mapping helper (in
`backend/actor.rs`) cover every `TaskStatus` variant, including that only
terminal states set `completed_at` and that `Failed`'s message carries
through.

**Headless in this PR — no GUI history list** (that's `F-HIST-002` below);
`BackendActor::download_history()` is a plain accessor + a startup log line
proving the data is live and durable, with nothing rendering it yet.

### F-HIST-002 — GUI download-history list (Shape-3 PR-2) · closed · MEDIUM
Render download history (`get_all_downloads()`/`delete_download()`, added by
`F-HIST-001`) in the GUI — a history view separate from the live queue list,
showing past downloads (including ones cleared from the active queue) with
their final status.

**Fix (PR [#35](https://github.com/ibra2000sd/rustloader/pull/35), open, not
yet merged):**
- **New `View::History`** — a third sidebar entry (alongside Downloads/
  Settings), matching the existing `settings_view`-style free-function view
  pattern (`src/gui/views/history_view.rs`, `src/gui/components/
  history_item.rs`). Lists persisted downloads newest-first (title, falling
  back to URL when empty; status; output path; file size or an explicit
  "Unknown size"; the completed/created timestamp), with a distinct
  loading/error/empty state.
- **Data access — direct, not routed through `BackendActor`:** `gui/app.rs`
  already holds the exact same `Arc<DatabaseManager>` the actor uses (#34);
  the History view reads/writes it directly via `Command::perform`, the same
  async pattern already established for Settings save/load. A
  `BackendCommand::GetHistory`/`BackendEvent::History` round-trip through the
  actor's channel would only add indirection with no benefit, since the GUI
  can already reach the same database the actor does.
- **Remove from history** deletes the DB record only (`delete_download`) —
  never the downloaded file — with an explicit label to that effect; optimistic
  local removal, reconciled by a reload on failure.
- **Show in Folder** reuses the exact `open::that(...)` call already shipped
  for the live queue's `OpenDownloadFolder` (`open` crate, already a
  dependency — no new one added); same detached, non-blocking, fire-and-forget
  behavior, just pointed at a `DownloadRecord`'s `output_path` instead of an
  active `DownloadTaskUI`'s.
- **Auto-refresh:** the existing status-diff logic in `BackendActor`'s
  `monitor_loop` already emits `DownloadCompleted`/`DownloadFailed`/
  `TaskStatusUpdated`; when one of those signals a terminal state AND the
  History view is the one currently open, the GUI reloads history — so a
  visible history list doesn't go stale while the user is looking at it,
  without polling.
- Re-download / open-file from history are explicit fast-follows, not in this
  PR.

No persistence/engine/queue/sidecar changes — `src/queue`/`src/downloader`
have zero diff; `database/operations.rs`'s only change is new tests (no new
or modified CRUD methods — `get_all_downloads`/`delete_download` already
existed from `F-HIST-001`). `download_segments` remains untouched/dead.

### B-DOC-002 — KNOWN_ISSUES.md content is stale · closed · SMALL
`B-DOC-001` fixed only the title's version stamp (now "v0.8.1"); the body was
still written around v0.1.1 and predated the whole download-reliability arc
(#28/#29/#30/#31). **Fix (PR [#32](https://github.com/ibra2000sd/rustloader/pull/32),
open, not yet merged):** rewrote the body against verified current-HEAD
evidence — resume scope (segmented-only, sidecar-guarded per F-DL-003;
`download_simple` and the yt-dlp/HLS path still don't resume), orphaned
`.partN` files on cancel (the F-DL-003 spinoff), aria2c's opt-in/not-yet-exposed
status (F-DL-001), the real cross-platform CI/release picture (builds+tests
pass on all 3 OSes; Windows/Linux release still "planned" per ROADMAP), and a
re-verified dependency-warning list (`cargo audit`: `instant`/`paste`/
`ttf-parser` unmaintained, `lru`/`memmap2` unsound, all transitive via `iced`).
Dropped two claims that no longer held: "82→15-20 compiler warnings" (a fresh
`cargo build` shows zero warnings from the crate's own code today) and
"limited unit test coverage" (186 tests pass across unit/integration/
persistence/stress suites). Added a "Recently Resolved (v0.8.x)" table for the
F-DL-002/B-DL-001/F-DL-003 fixes. Docs-only; no Rust source touched.
Source: internal audit 2026-06-30.

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
| `F-DL-003` | Cross-session resume sidecar identity guard | PR #30, `f51dfad`, 2026-07-01 |
| `F-DL-001` | Opt-in aria2c external downloader for yt-dlp path | PR #31, `38ea148`, 2026-07-01 |
| `B-DOC-002` | KNOWN_ISSUES.md content refresh | PR #32, 2026-07-01 (PR pending) |

(Pre-`docs/ai-os` work was tracked via GitHub PRs/CHANGELOG; future items use the
IDs above.)
