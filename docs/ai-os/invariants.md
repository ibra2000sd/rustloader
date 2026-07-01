# Invariants

Things that must stay true across changes. Breaking one is either a bug or a
deliberate decision that needs an ADR. Each is grounded in current code at
`1c038e2`.

## I-1 — Every external-process call is timeout-bounded and kills on timeout
No `Command::output()` / `child.wait()` may await a spawned binary without an
upper bound and a guaranteed kill. Current compliant sites:
- Extraction: all four yt-dlp calls go through `run_bounded(cmd,
  EXTRACTION_TIMEOUT=60s)` with `kill_on_drop(true)` (`extractor/ytdlp.rs`,
  PR #23).
- Download: `download_via_ytdlp` uses `timeout(1800s, child.wait())` +
  `child.kill()` (`downloader/engine.rs`).
- Probe: ranged GET wrapped in `timeout(10s, …)`.
Any new subprocess (e.g. an aria2 integration) inherits this rule. See
[`adr/0003-bound-all-external-process-calls.md`](adr/0003-bound-all-external-process-calls.md).

## I-2 — The GUI never drives the engine directly
All GUI→work flow is `BackendCommand` over the channel to `BackendActor`; all
work→GUI flow is `BackendEvent`. Don't call engine/queue methods from GUI code, and
don't block the GUI thread on async work.

## I-3 — The progress/command contract is stable
Any download backend must **emit** `DownloadProgress` (fields per
`progress.rs`) at ~1s cadence and **accept** pause/resume/cancel with the current
semantics. A backend that downgrades progress granularity or pause/resume fidelity
is a **regression**, not a neutral swap.

## I-4 — `QueueManager` is the single source of truth for task state
Task status lives in the queue FSM (`TaskStatus`), not scattered across GUI/engine.
Transitions go through `QueueManager` and are logged to the `EventLog`.

## I-5 — Locking hierarchy: queue (L2) before active_downloads (L1)
Acquire the queue lock before the `active_downloads` lock, never the reverse.
This ordering is the documented deadlock-avoidance contract (roadmap v0.4).

## I-6 — Event log is append-only and corruption-tolerant
`queue/events.rs` is JSONL append-only; `rehydrate()` must skip malformed lines
and continue. Previously-`Started` tasks rehydrate as `Paused` (no auto-blast on
restart). Don't introduce in-place rewrites of the log.

## I-7 — Cookie args come from one place
yt-dlp cookie flags are produced only by `CookieConfig` and applied identically to
every yt-dlp invocation (extraction and download). Don't hand-assemble
`--cookies*` flags at a call site. `default()` must keep emitting nothing.

## I-8 — Never write a non-media response as a media file
The Content-Type routing guard (PR #14) refuses to save an HTML/non-media response
as the output media file. Preserve this guard.

## I-9 — Third-party binaries are integrated at arms-length (external subprocess)
External tools (yt-dlp today; aria2 if adopted) are invoked as **separate
processes** located via bundle/PATH detection — not linked or statically
incorporated. Copyleft (GPL) binaries are **not bundled** into the distribution.
See [`adr/0002-external-subprocess-no-gpl-bundling.md`](adr/0002-external-subprocess-no-gpl-bundling.md).

## I-10 — CI is the gate, on three OSes
A change is not "done" until `cargo test --all`, `cargo clippy --all-targets
--all-features -- -D warnings`, `cargo fmt --all -- --check`, and `cargo audit`
pass on ubuntu/macOS/windows. Clippy runs `-D warnings`.

## I-11 — No fabricated SHAs, CI results, or test output
Documented prior failure mode. Every claimed SHA/CI/test result is backed by a
real command. (Operating rule #2 in CLAUDE.md, repeated here because it is a
correctness invariant for this project, not just etiquette.)

---

### Known invariant GAP (not yet true, tracked)
- **Cross-session resume correctness is not fully an invariant yet.**
  `segment.rs`'s per-attempt resume (F-DL-002/#28) plus the `206`-required
  guard (B-DL-001/#29) mean segment writes do **not** unconditionally truncate
  any more, and — as a side effect of `.partN` files surviving pause/cancel/
  app-close and `calculate_segments` being deterministic — resume does **not**
  unconditionally restart from zero either; both of those older claims were
  stale as of the F-DL-003 spike (2026-07-01). What was missing, and what
  F-DL-003 (Shape 2, PR #30) adds, is the *safety guard*: a sidecar identity
  check (URL + file_size + segment_count) that must match before any existing
  part is trusted, so a segment-count change between sessions or a different
  download reusing the same `output_path` can no longer silently corrupt the
  output. Until #30 merges, do not assume cross-session resume validates
  identity — the un-gated version is on `main` today.
