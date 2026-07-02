# Status

> Update at the end of any session that lands work. This file — not `ROADMAP.md`
> or `README.md` — is the live source of truth for "where are we".

**As of:** 2026-07-02
**Released version:** v0.8.1 (first published release, 2026-06-29); **v0.9.0
staged** — version bump + docs landed via the release-prep PR (branched from
`e6cd092`), tag/release execution still pending (maintainer action)
**main HEAD:** `e6cd092` (the #40 merge, quick-xml audit-ignore; supersedes the
earlier `81d4dad`/#38 stamp — #39 and #40 have since merged)
**CI on main:** green (4 jobs × ubuntu/macOS/windows)
**Open PRs:** #1 (draft, untouched), plus this v0.9.0 release-prep PR
(docs + version only, branched from `e6cd092`)

## Where the project is

Per the project's own `ROADMAP.md`, the foundational stages are **done**:
multi-segment engine + Iced GUI + yt-dlp + SQLite (v0.1), security hardening
(v0.1.1), actor model (v0.2), event sourcing + EventLog (v0.3), Queue Manager FSM
(v0.4), concurrency hardening (v0.5). v0.8.1 is the first **published** release,
adding Content-Type routing, the resilient segmented engine, authenticated-site
cookie support, and the extraction-timeout fix.

## Current focus: download reliability

The recent arc has been extraction reliability; the last few sessions were
**download reliability** (the two defects the aria2 spike localized), both now
closed:
1. one failed segment aborting the whole transfer (throttled-host failure) —
   fixed, `F-DL-002`/#28 (the retry-resume half; the engine's `break`-on-
   segment-failure itself remains open if still wanted).
2. cross-session resume — fixed, `F-DL-003`/#30 (sidecar identity guard on top
   of the byte-level resume #28/#29 already provided).

`F-DL-001` (Shape A: opt-in aria2c for the yt-dlp download path) also landed,
shipped default-off after live-testing found a real progress-contract
regression for the one case it would have changed anything (see "Done" below).

## Done (recent)

- **v0.9.0 release prep** (2026-07-02, base `e6cd092`, PR pending) — staged
  the release the read-only audit recommended, docs + version only (no `src/`
  changes): `Cargo.toml` 0.8.1 → 0.9.0 (+ lockfile), a dated 0.9.0
  `CHANGELOG.md` entry covering #19–#40 with fixed link refs,
  `KNOWN_ISSUES.md` made true at HEAD (ISSUE-003 resolved via #36, ISSUE-004
  corrected for `--experimental-aria2c`, ISSUE-005's ~90 MB corrected to the
  measured ~6.9 MB, ISSUE-006 updated for the #40 audit ignores, ISSUE-002
  resolved by this release; new entries for the missing disk-space pre-check,
  resume-progress under-reporting, and unsigned binaries), a README truth
  pass (0.9.0 badge/status, real resume claims, ~6.9 MB, three-platform
  Releases install + ffmpeg/JS-runtime prerequisites + Gatekeeper/SmartScreen
  first-run steps), ROADMAP 0.8/0.9 sections brought to reality, and
  `RELEASE_NOTES.md` replaced (was stale at v0.1.1) with a v0.9.0 summary.
  The `v0.9.0-rc.1` dry-run, tag, and Windows/Linux artifact smoke-test are
  the next, separate release-execution steps (need a human/VM).
- **B-DL-006 — saved extension now reflects the actual content** (2026-07-02,
  base `81d4dad`, merged `0ae844a` via PR #39) — master-audit finding 3. The caller's
  mode-derived extension (`.mp3`/`.mp4` from the CLI flag; the GUI's hardcoded
  `.mp4`) is now provisional: the engine finalizes it from what was actually
  fetched (probe `Content-Type` → redirect-resolved URL extension → caller's
  extension) and `DownloadEngine::download` returns the real saved path; the
  yt-dlp path uses a `%(ext)s` output template and adopts the file yt-dlp
  actually wrote. Verified live against the audit's own repros: an
  `audio/mpeg` file now saves `.mp3` (was `.mp4`), an octet-stream installer
  saves `.exe` (was `.mp4`), and a yt-dlp-fallback download gets yt-dlp's real
  extension. `.partN`/sidecar naming, #36 cleanup, F-DL-003 resume identity,
  I-3 progress, and the I-8 non-media guard are unchanged.
- **F-EXTRACT-001 Phase-0 desk spike** (2026-07-02, base `9eaee55`) — read-only
  proxy-capture (res-downloader style) feasibility. Resolved the stale-snapshot
  divergence (local tree = `71c463c`; authoritative = origin/main `9eaee55`);
  live-verified the crate/license landscape and chose **`hudsucker`** (MIT/Apache)
  as the MITM base, **rejecting `slinger-mitm`** on license (GPL-3.0-only, ADR 0002).
  Findings drafted (`docs/ai-os/spikes/F-EXTRACT-001-phase0-findings.md`) and
  **ADR 0004** written (`adr/0004-proxy-capture-mitm.md`, Status: **Proposed**,
  gated). The machine-level proofs (root-CA install, system-proxy rewrite, real-site
  pinning) are **deferred to a supervised session** — not run unattended, not
  fabricated. `F-EXTRACT-001` stays **open**. Docs-only; no Rust source touched.
- **PR #21** (`dff16f2`) — float-`duration` deserialize fix (SoundCloud).
- **PR #22** (`933b2c0`) — yt-dlp default format selector fixes HLS master.
- **PR #23** (`1c038e2`) — extraction subprocess now timeout-bounded + kill
  (`run_bounded`, `EXTRACTION_TIMEOUT=60s`). Mechanism proven by unit tests;
  live timeout-firing in the GUI remains maintainer-smoke-only.
- **aria2 adoption spike** (read-only audit, 2026-06-30) — localized both download
  defects, found `enable_resume` is dead config, confirmed the yt-dlp download
  path is already bounded, confirmed aria2 = GPL-2.0 (→ external-only, no bundle),
  and confirmed the README/LICENSE inaccuracies. Recommendation: Shape A (cheap
  yt-dlp-path win) + fix download reliability in the native engine (Shapes
  D1/D2) over adopting aria2 for the native path.
- **docs/ai-os pack + CLAUDE.md** — this pack (2026-06-30).
- **`.claude/skills` layer** (2026-07-01) — vendored the audited
  `leonardomso/rust-skills` skill (MIT) and authored
  `rustloader-invariants-guard`, which turns `docs/ai-os/invariants.md` into
  an actionable per-diff checklist. Docs/skills-only; no Rust source touched.
- **B-DOC-001** (2026-07-01) — added a real `LICENSE` (MIT) + `Cargo.toml
  license = "MIT"`, enacting `adr/0001` (now Accepted); corrected the README's
  resume claims to match the verified code (restart-on-resume, no byte-level
  resume yet); fixed the `ROADMAP.md`/`KNOWN_ISSUES.md` stale version stamps to
  v0.8.1. Docs/metadata-only; no Rust source touched.
- **F-DL-002** (2026-07-01, PR [#28](https://github.com/ibra2000sd/rustloader/pull/28), merged
  `c1c0580`) —
  segment retries in `segment.rs` now resume from already-written bytes
  (ranged `Range: bytes={start+W}-{end}` + append, cumulative progress,
  wall-clock-bounded retry budget) instead of `File::create`-truncating on
  every attempt. Fixes the throttled-host failure mode where a dropped
  connection kept hitting the same point and never completed. The engine's
  `break` on a genuinely-unrecoverable segment is intentionally unchanged.
  Cross-session resume safety was fixed by `F-DL-003` (PR #30, below).
- **B-DL-001** (2026-07-01, PR [#29](https://github.com/ibra2000sd/rustloader/pull/29), merged
  `c976872`) —
  closed a silent-corruption gap in #28's resume path: `segment.rs` now
  requires `206 Partial Content` (not just any 2xx) before appending on a
  resume attempt; a `200 OK` (server/proxy ignoring `Range`) truncates the
  stale `.partN` and restarts the segment fresh instead of appending a full
  body onto the partial bytes. New regression test
  `test_resume_restarts_when_server_ignores_range`; all #28 tests still pass.
- **F-DL-003 spike** (read-only, 2026-07-01) — determined cross-session resume
  is neither a pure wire-up nor a ground-up build: the `download_segments`/
  `downloads` DB tables are fully dead (zero callers outside
  `database/operations.rs`), but #28/#29 already make byte-level resume happen
  *unintentionally* across pause/cancel/app-close (no `.partN` cleanup on
  interruption + deterministic `calculate_segments` + preserved `output_path`/
  URL). The real gap is a missing safety guard, not missing mechanism — two
  concrete silent-corruption paths identified: a segment-count preference
  change between sessions, and a different download reusing the same
  `output_path`. Recommended Shape 2 (filesystem sidecar identity check).
- **F-DL-003** (2026-07-01, Shape 2, PR
  [#30](https://github.com/ibra2000sd/rustloader/pull/30), merged `f51dfad`) —
  added `downloader::resume_guard`: a `<output>.rustloader-resume` sidecar
  recording `{url_hash, file_size, segment_count}`, written before segment
  downloads start and checked on every `download()` call. An existing
  `.partN` set is trusted only when the sidecar matches (URL + file_size +
  segment_count) and `enable_resume` is `true`; any mismatch, a missing
  sidecar with parts present, or resume disabled discards the parts and
  restarts clean. `enable_resume` now gates real behavior for the first time.
  Four new regression tests against a real mock HTTP server exercise:
  matching-identity resume (partial re-fetch), segment-count-changed (clean
  restart, byte-correct), foreign-download-reuses-path (clean restart,
  byte-correct, not the foreign bytes), and `enable_resume=false` (ignores
  even fully matching parts). All #28/#29 tests still pass.
- **F-DL-001** (2026-07-01, Shape A, PR
  [#31](https://github.com/ibra2000sd/rustloader/pull/31), merged `38ea148`) — added
  `extractor::ytdlp::find_aria2c` (mirrors `find_ytdlp`/
  `find_in_common_paths`, deliberately without the bundled-binary check — I-9)
  and a `YtDlpOptions::use_aria2c` opt-in (default `false`) that makes
  `build_ytdlp_args` add `--downloader aria2c` when both are true. **Shipped
  opt-in, not auto-enabled on detection**, after live-smoke-testing real
  `yt-dlp --downloader aria2c` runs (HLS test stream + a direct HTTP file)
  against yt-dlp 2026.06.09 + aria2 1.37.0 and reading yt-dlp's own
  `downloader/external.py`: (1) aria2c never engages for HLS/DASH at all
  (`Aria2cFD.SUPPORTED_PROTOCOLS` excludes `m3u8`/`dash` — zero benefit for
  `download_via_ytdlp`'s primary use case) and (2) for the plain
  http/https/ftp transfers it does take over, yt-dlp's `ExternalFD` only
  hooks progress once, on completion — the progress bar would sit frozen at
  0% for the whole transfer. No CLI/GUI toggle wired up yet (nothing sets
  `use_aria2c: true`), so this PR is a no-op by default — confirmed via
  `cargo run -- <url> --dry-run` producing byte-identical args to before.
- **B-DOC-002** (2026-07-01, PR
  [#32](https://github.com/ibra2000sd/rustloader/pull/32), merged `d3f0035`) —
  `KNOWN_ISSUES.md`'s body was still written around v0.1.1 (resolved-bugs
  table from that release, "Rustloader version (v0.1.1)" in the reporting
  section) and predated the whole download-reliability arc. Rewrote it
  against verified current-HEAD evidence: resume scope (segmented-only,
  sidecar-guarded; `download_simple` and the yt-dlp/HLS path still don't
  resume), orphaned `.partN` files on cancel, aria2c's opt-in/not-yet-exposed
  status, the real cross-platform CI/release picture, and a re-verified
  `cargo audit` dependency list. Dropped the stale "82→15-20 compiler
  warnings" and "limited test coverage" claims (a fresh build shows zero
  warnings from the crate's own code; 186 tests now pass across
  unit/integration/persistence/stress suites). Docs-only; no Rust source
  touched.
- **F-DL-001b** (2026-07-01, PR
  [#33](https://github.com/ibra2000sd/rustloader/pull/33), merged `29b3ff1`) —
  wired the
  actual enable path for #31's dormant `use_aria2c`: a new
  `--experimental-aria2c` CLI flag (default `false`), threaded straight to
  `YtDlpOptions::use_aria2c`. Help text labels it `EXPERIMENTAL` and states
  the progress-freeze caveat up front. No GUI checkbox added (no existing
  settings-UI widgets to attach an advanced/experimental option to without
  substantially more work than this task's scope). Verified both directions
  with real `cargo run -- <url> --dry-run` calls: absent the flag, args are
  byte-identical to before (no `--downloader`); with the flag and a real
  `aria2c` installed, args include `--downloader aria2c`. New tests: two unit
  tests in `cli.rs` (flag → `use_aria2c` mapping, default-off), three
  binary-level tests in `tests/cli_test.rs` (help mentions the flag and labels
  it experimental; dry-run omits `--downloader` by default; dry-run adds it
  when the flag is set and aria2c is actually present, environment-dependent
  like the existing `find_aria2c`/`find_ytdlp` smoke tests).
- **F-HIST-001** (2026-07-01, Shape-3 PR-1, PR
  [#34](https://github.com/ibra2000sd/rustloader/pull/34), merged `04e0148`) — wired the
  previously-dead `downloads` table into the download lifecycle as a durable
  history (`download_segments` stays dead — resume is still the sidecar's
  job, not this table's). `BackendActor` now holds the SAME `DatabaseManager`
  Arc `gui/app.rs` already builds for settings; `downloads.id` = the queue
  task UUID, reconciling 1:1 with `EventLog`/`DownloadTask.id`. A row is
  inserted (status `Queued`) right after `queue_manager.add_task` succeeds in
  `handle_start_download`, and updated in place (`INSERT OR REPLACE`) on
  every subsequent status transition via the existing `monitor_loop` polling
  diff — no new detection mechanism, `queue/manager.rs` and `queue/events.rs`
  have zero diff (I-4/I-6 both structurally unaffected). Removing a task from
  the live queue does not delete its history row. New tests:
  `download_history_survives_reopening_the_database` (writes rows, opens a
  brand-new pool against the same file, reads them back correctly — the
  literal restart-persistence bar) and
  `status_transitions_update_in_place_not_duplicate` (three transitions of
  one task id leave exactly one row); plus unit tests for the new
  `task_status_db_fields` status-mapping helper. Headless — no GUI history
  list yet (`F-HIST-002`, below); `BackendActor::download_history()` is a
  plain accessor + a startup log line proving the data is live and durable.
- **F-HIST-002** (2026-07-01, Shape-3 PR-2, PR
  [#35](https://github.com/ibra2000sd/rustloader/pull/35), merged `59d96bb`) — a visible
  History view: a third sidebar entry listing persisted downloads
  newest-first (title/URL fallback, status, output path, size, timestamp),
  with Remove-from-history (`delete_download` — DB record only, not the
  file, labelled as such) and Show-in-Folder (reuses the existing
  `open::that(...)` call already shipped for the live queue, no new
  dependency). Reads/writes the GUI's existing `Arc<DatabaseManager>`
  directly via `Command::perform` (the same pattern already used for
  Settings) rather than adding a `BackendCommand`/`BackendEvent` round-trip —
  simpler, since the GUI already holds the same database the actor does.
  Auto-refreshes when a download reaches a terminal state while the History
  view is open, reusing the existing `monitor_loop` status-diff (no polling
  added). `src/queue`/`src/downloader` have zero diff; `database/
  operations.rs` only gained tests (`delete_download_removes_only_the_
  targeted_record`), no new/changed CRUD. New pure-function tests for the
  title/size/timestamp row mapping. Manual smoke: launched the real compiled
  binary — confirmed clean startup (real Metal-backed window, real local
  database read logging "Loaded 16 historical download record(s)" from
  actual prior-usage data, no panics/errors, clean shutdown) but could not
  get a pixel-level screenshot, since the raw dev binary isn't a bundled
  `.app` macOS's permission system can target — see the PR for the full
  account, including a safety correction (an initial test-data insert into
  the real shared local database was caught and cleaned up immediately;
  a second attempt was correctly blocked).
- **B-DL-002** (2026-07-02, PR
  [#36](https://github.com/ibra2000sd/rustloader/pull/36), merged `612bda9`) — the F-DL-003
  hygiene spinoff: `cancel_task`/`remove_task` (`queue/manager.rs`) now
  best-effort delete the task's `<output>.partN` files and its
  `<output>.rustloader-resume` sidecar via a new private
  `cleanup_task_artifacts` helper (sidecar via `resume_guard`'s
  `sidecar_path`/`remove_sidecar`; parts via a strict
  `<file_name>.part<digits>` scan of the output's directory, since the
  segment count isn't known at the queue layer). Cleanup runs after the
  queue/active locks are released (I-5 respected); failures are logged, never
  propagated. `pause_task` deliberately unchanged — parts + sidecar must
  survive pause or cross-session resume (F-DL-003/#30) breaks;
  `clear_completed` needs nothing (the engine already removes both at merge
  time). Engine/resume/sidecar/persistence untouched. New regression tests
  in `tests/orphan_cleanup_test.rs`: cancel and remove delete the artifacts
  (decoy files untouched), and — the load-bearing guard — pause keeps them.
- **B-DL-004 + B-DL-005** (2026-07-02, PR
  [#37](https://github.com/ibra2000sd/rustloader/pull/37), open) — the two
  B-class native-path bugs from the 2026-07-01/02 **master audit** (an
  isolated end-to-end capability test of the real binary at `612bda9`).
  B-DL-004: the engine's reqwest client used `.timeout(30s)` — a TOTAL
  request timeout including the body read — so any native transfer whose
  body took >30s failed (`download_simple` could never complete on a slow
  link; segmented downloads churned through forced retries every 30s, kept
  alive only by #28/#29's resume-append). Replaced with `CONNECT_TIMEOUT`
  (15s, handshake only) plus a new `STALL_ABORT_TIMEOUT` (aligned with
  `STALL_DETECTION_SECONDS` = 30s) bounding every native wait — response
  headers and each body-chunk read in both native paths — so
  slow-but-progressing transfers complete while idle/dead connections still
  abort within a bounded window (I-1's spirit applied to HTTP reads; the
  stall watchdog stays notify-only). B-DL-005: `download_simple` wrote the
  final path directly and left a corrupt-looking partial under the final
  name on failure; it now streams to `<file_name>.part0` (the existing
  segment naming, already covered by #36's cancel/remove cleanup and the
  resume-guard discard) and renames on success, removing the temp on
  failure. Verified with a real before/after repro against a local ~70 KB/s
  no-range server (main: died at 65.9% + left a 2,085,888-byte partial;
  branch: completed byte-correct, sha256-verified) + 5 new regression tests;
  full local gates green (210 tests, clippy `-D warnings`, fmt, audit).

## Next (ordered)

1. **F-HIST-002 follow-ups** — re-download / open-file directly from
   history (explicit fast-follows, not done in #35); a real screenshot-based
   visual pass by the maintainer, since the automated smoke couldn't get one
   in this environment.
2. **F-DL-001b follow-up** — decide whether the progress-hook gap for
   aria2c-driven http/https/ftp transfers is worth addressing (parsing
   aria2c's own progress format), and whether/how to expose
   `--experimental-aria2c` in the GUI once there's an advanced-settings area
   to put it in.
3. **F-DL-002 (remaining half)** — the engine's `break`-on-segment-failure
   still aborts the whole download; the retry-resume half is done (#28), the
   whole-download tolerance half remains open if still wanted.

## Open product directions (maintainer decides)

- Cross-platform polish for Windows/Linux (CHANGELOG targets v0.9.0).
- Proxy-capture / browser-extension capture (CHANGELOG targets v1.0.0;
  `F-EXTRACT-001`) — gated on the legitimate-use scope decision.
- License: resolved — MIT adopted (`adr/0001`, Accepted).
