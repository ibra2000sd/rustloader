---
name: rustloader-invariants-guard
description: Before marking any rustloader change complete, or when reviewing a diff/PR, to check it against the project invariants (docs/ai-os/invariants.md).
---

# rustloader-invariants-guard

rustloader is a single-binary desktop download manager (Iced GUI + Tokio,
wrapping `yt-dlp`). This skill turns
[`docs/ai-os/invariants.md`](../../../docs/ai-os/invariants.md) into a fast,
actionable per-diff checklist.

**`docs/ai-os/invariants.md` is the canonical source.** This checklist is an
operational index over it, not a replacement — if the two ever disagree, the
canonical file wins. Whenever an invariant in `invariants.md` changes, update
this file **in the same PR** (per `CLAUDE.md`'s operating rules).

## When to use this skill

- Before marking any rustloader change complete.
- When reviewing a diff or PR against `main`.
- Any time a change touches subprocesses, the GUI/backend boundary, the
  download/progress path, the queue, locking, the event log, cookies,
  content-type handling, third-party binaries, or CI/release claims.

## How to use it

1. Run `git diff main...HEAD --stat` (or read the PR diff) to see which files
   changed.
2. Walk the checklist below. Skip an invariant only if the diff provably does
   not touch the files/behavior it governs — say so explicitly, don't assume.
3. For every invariant that *is* touched, run the listed verification action
   and report the result (command output, not a guess).
4. Do not mark the change complete until every touched invariant has a
   recorded pass. If one fails, fix it or escalate — don't silently proceed.

## Checklist (I-1 … I-11)

### I-1 — Every external-process call is timeout-bounded and kills on timeout
**Touches:** any new/modified `Command`, `child.wait()`, or subprocess spawn
(e.g. an aria2 integration, a new yt-dlp invocation site).
**Check:** `grep -n "Command::new\|\.spawn()\|\.output()\|\.wait()" <changed files>`
and confirm every match is routed through a bounded helper (e.g.
`run_bounded`/`tokio::time::timeout`) with `kill_on_drop(true)` or an explicit
`child.kill()`. A subprocess call with no timeout and no guaranteed kill is a
violation.

### I-2 — The GUI never drives the engine directly
**Touches:** any change under `gui/` that calls into `downloader/`, `queue/`,
or `extractor/` directly, or any change that blocks the GUI thread on async
work.
**Check:** `grep -rn "downloader::\|queue::\|extractor::" src/gui/` — new hits
are a violation unless they go through a `BackendCommand` sent to
`BackendActor`. GUI code should only read `BackendEvent`s from the
subscription.

### I-3 — The progress/command contract is stable
**Touches:** any new or modified download backend.
**Check:** confirm the backend still emits `DownloadProgress` (fields per
`downloader/progress.rs`) at ~1s cadence, and that pause/resume/cancel
semantics are unchanged or improved. A backend that drops progress
granularity or pause/resume fidelity is a regression, not a neutral swap —
flag it even if tests pass.

### I-4 — `QueueManager` is the single source of truth for task state
**Touches:** any change that tracks or mutates download/task status outside
`queue/manager.rs`.
**Check:** `grep -rn "TaskStatus" src/` outside `queue/` — task state should
flow through `QueueManager` transitions, which are logged to the `EventLog`,
not set ad hoc in GUI/engine code.

### I-5 — Locking hierarchy: queue (L2) before active_downloads (L1)
**Touches:** any change that acquires both the queue lock and the
`active_downloads` lock.
**Check:** read the new/modified function and confirm the queue lock is
acquired first in every code path, never the reverse. A reversed acquisition
order is a deadlock risk even if it "works" in testing.

### I-6 — Event log is append-only and corruption-tolerant
**Touches:** any change to `queue/events.rs` or `rehydrate()`.
**Check:** confirm writes are append-only (no in-place rewrite/seek-and-edit
of the log file), and that `rehydrate()` still skips malformed lines instead
of erroring out. Confirm previously-`Started` tasks still rehydrate as
`Paused` (no auto-blast on restart).

### I-7 — Cookie args come from one place
**Touches:** any change that builds yt-dlp command-line args.
**Check:** `grep -rn -- "--cookies" src/` — every match should come from
`CookieConfig` (`utils/cookies.rs`), applied identically to extraction and
download call sites. A hand-assembled `--cookies*` flag at a call site is a
violation. Confirm `CookieConfig::default()` still emits nothing.

### I-8 — Never write a non-media response as a media file
**Touches:** any change to the download path's response handling
(`downloader/engine.rs` and friends).
**Check:** confirm the Content-Type routing guard (PR #14) is still in place
and still refuses to save an HTML/non-media response as the output media
file. If the guard's condition changed, re-verify it against a non-media
`Content-Type` test case.

### I-9 — Third-party binaries are integrated at arms-length (external subprocess)
**Touches:** any change that adds or links a new third-party binary
(yt-dlp today; aria2 or similar if adopted).
**Check:** confirm the binary is invoked as a separate process located via
bundle/PATH detection (mirroring `utils/bundle_paths.rs` / `utils/depcheck.rs`),
not statically linked or vendored into the binary. If the new dependency is
GPL-licensed, confirm it is not bundled into the distribution — see
[`adr/0002-external-subprocess-no-gpl-bundling.md`](../../../docs/ai-os/adr/0002-external-subprocess-no-gpl-bundling.md).

### I-10 — CI is the gate, on three OSes
**Touches:** every change.
**Check:** before marking complete, actually run `cargo test --all`, `cargo
clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all --
--check`, and `cargo audit` locally and report the real output. A single new
clippy warning fails CI (`-D warnings`). Don't claim "CI is green" without a
command you ran backing it up.

### I-11 — No fabricated SHAs, CI results, or test output
**Touches:** every change, every report back to the user.
**Check:** every SHA cited comes from a real `git rev-parse`/`git log` you
ran; every "tests pass" claim comes from a real `cargo test` run you can
show; every `file:line` reference comes from a file you actually opened. If
you didn't run it, don't claim it.

## Notes on scope

- A change can legitimately touch zero invariants (e.g. a doc typo fix) — say
  so rather than padding the checklist.
- A change can touch several invariants at once (e.g. an aria2 integration
  would implicate I-1, I-3, and I-9 together) — check each independently.
- The "Known invariant GAP" section of `invariants.md` (byte-level resume is
  NOT yet true) is not a checklist item here, but don't let a change silently
  assume resume works — that's still tracked as `F-DL-003` in the backlog.
