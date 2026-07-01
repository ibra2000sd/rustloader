# Status

> Update at the end of any session that lands work. This file — not `ROADMAP.md`
> or `README.md` — is the live source of truth for "where are we".

**As of:** 2026-07-01
**Released version:** v0.8.1 (first published release, 2026-06-29)
**main HEAD:** `c1c0580` (the #28 merge; previous stamp `3494b3c`/#27 was stale)
**CI on main:** green (4 jobs × ubuntu/macOS/windows)
**Open PRs:** #1 (draft, untouched), [#29](https://github.com/ibra2000sd/rustloader/pull/29)
(B-DL-001 resume-requires-206 fix, open, branched from `c1c0580`)

## Where the project is

Per the project's own `ROADMAP.md`, the foundational stages are **done**:
multi-segment engine + Iced GUI + yt-dlp + SQLite (v0.1), security hardening
(v0.1.1), actor model (v0.2), event sourcing + EventLog (v0.3), Queue Manager FSM
(v0.4), concurrency hardening (v0.5). v0.8.1 is the first **published** release,
adding Content-Type routing, the resilient segmented engine, authenticated-site
cookie support, and the extraction-timeout fix.

## Current focus: download reliability

The recent arc has been extraction reliability; the next arc is **download
reliability** (the two defects the aria2 spike localized):
1. one failed segment aborting the whole transfer (throttled-host failure), and
2. no byte-level resume (interrupted downloads restart from zero).

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
- **F-DL-002** (2026-07-01, PR [#28](https://github.com/ibra2000sd/rustloader/pull/28), open) —
  segment retries in `segment.rs` now resume from already-written bytes
  (ranged `Range: bytes={start+W}-{end}` + append, cumulative progress,
  wall-clock-bounded retry budget) instead of `File::create`-truncating on
  every attempt. Fixes the throttled-host failure mode where a dropped
  connection kept hitting the same point and never completed. The engine's
  `break` on a genuinely-unrecoverable segment is intentionally unchanged.
  Cross-session/true byte-resume (`F-DL-003`) remains not-yet-true.
- **B-DL-001** (2026-07-01, PR [#29](https://github.com/ibra2000sd/rustloader/pull/29), open) —
  closed a silent-corruption gap in #28's resume path: `segment.rs` now
  requires `206 Partial Content` (not just any 2xx) before appending on a
  resume attempt; a `200 OK` (server/proxy ignoring `Range`) truncates the
  stale `.partN` and restarts the segment fresh instead of appending a full
  body onto the partial bytes. New regression test
  `test_resume_restarts_when_server_ignores_range`; all #28 tests still pass.

## Next (ordered)

1. **F-DL-001** — Shape A: `yt-dlp --downloader aria2c` (external aria2c only).
   Unblocked now that `B-DOC-001` has landed.
2. **F-DL-002** — segment-failure tolerance (don't abort whole download; fix retry
   truncation). Fixes the throttled-host failure.
3. **F-DL-003** — byte-level resume + checkpoint (investigate the existing
   `download_segments` table first — may be wire-up, not build).

## Open product directions (maintainer decides)

- Cross-platform polish for Windows/Linux (CHANGELOG targets v0.9.0).
- Proxy-capture / browser-extension capture (CHANGELOG targets v1.0.0;
  `F-EXTRACT-001`) — gated on the legitimate-use scope decision.
- License: resolved — MIT adopted (`adr/0001`, Accepted).
