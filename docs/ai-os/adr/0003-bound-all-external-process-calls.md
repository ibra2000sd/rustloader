# ADR 0003 — All external-process calls must be timeout-bounded

**Status:** Accepted
**Date:** 2026-06-30

## Context
The extractor awaited `yt-dlp` via `.output().await` with no timeout. Any hang
(stalled network, unresponsive site, locked browser-cookie DB) froze the GUI
indefinitely. PR #23 fixed the four extraction call sites. The download path
already had a bounded wait (`timeout(1800s, child.wait())` + `child.kill()`), and
the probe is bounded (10s). The lesson generalizes: an unbounded await on a
spawned process is a latent freeze.

A subtlety: wrapping `Command::output()` in `tokio::time::timeout` does **not**
kill the child on elapse unless `kill_on_drop(true)` is set (or the child is
spawned and killed explicitly). A timeout without a kill leaks an orphan process
(which, for cookie reads, may hold a browser DB lock).

## Decision
Every place that spawns and awaits an external process MUST bound the wait and
guarantee the process is killed on timeout — either `kill_on_drop(true)` +
`timeout(dur, cmd.output())`, or `spawn()` + `timeout(dur, child.wait_with_output())`
+ explicit `child.kill().await`. Timeout durations are named constants, not inline
literals.

## Consequences
- No new code path may reintroduce an unbounded process await. Reviewers check this
  on any change that adds a `Command`/subprocess.
- An aria2 integration (`F-DL-001`) inherits this rule.

## Invariant
Recorded as `I-1` in `invariants.md`.
