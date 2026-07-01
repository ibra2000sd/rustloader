# Status

> Update at the end of any session that lands work. This file — not `ROADMAP.md`
> or `README.md` — is the live source of truth for "where are we".

**As of:** 2026-07-01
**Released version:** v0.8.1 (first published release, 2026-06-29)
**main HEAD:** `f51dfad` (the #30 merge; previous stamp `c976872`/#29 was stale)
**CI on main:** green (4 jobs × ubuntu/macOS/windows)
**Open PRs:** #1 (draft, untouched), [#31](https://github.com/ibra2000sd/rustloader/pull/31)
(F-DL-001 opt-in aria2c downloader, open, branched from `f51dfad`)

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
  [#31](https://github.com/ibra2000sd/rustloader/pull/31), open) — added
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

## Next (ordered)

1. **F-DL-003 spinoffs** — orphaned-`.partN` cleanup on cancel/remove
   (`queue/manager.rs`); optionally, Shape 3's DB-backed plan persistence as a
   later enhancement over the filesystem sidecar.
2. **F-DL-001 follow-up** — decide whether/how to expose `use_aria2c` as a
   real opt-in (CLI flag or GUI setting), and whether the progress-hook gap
   for aria2c-driven http/https/ftp transfers is worth addressing (parsing
   aria2c's own progress format, or accepting frozen-then-jump progress for
   that narrow case) before doing so.
3. **F-DL-002 (remaining half)** — the engine's `break`-on-segment-failure
   still aborts the whole download; the retry-resume half is done (#28), the
   whole-download tolerance half remains open if still wanted.

## Open product directions (maintainer decides)

- Cross-platform polish for Windows/Linux (CHANGELOG targets v0.9.0).
- Proxy-capture / browser-extension capture (CHANGELOG targets v1.0.0;
  `F-EXTRACT-001`) — gated on the legitimate-use scope decision.
- License: resolved — MIT adopted (`adr/0001`, Accepted).
