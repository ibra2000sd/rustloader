# CLAUDE.md — operating rules for AI sessions on rustloader

This file is read first by any AI assistant (Claude Code, etc.) working in this
repo. It encodes the working agreement. The detailed maps live in
[`docs/ai-os/`](docs/ai-os/README.md).

rustloader is a cross-platform desktop **video/file download manager** written in
Rust (Iced GUI + Tokio), wrapping `yt-dlp` for extraction and a native
multi-segment HTTP engine for downloading. It is a single-binary desktop app —
**not** a web service. There is no multi-tenant layer, no server, no production
URL; "production" means the released binary.

## Non-negotiable working rules

1. **Investigate before editing.** Verify every claim — yours, the user's, or a
   prior session's summary — against the actual tree at the current HEAD before
   changing anything. Framing is a hypothesis; the source is ground truth.

2. **No fabrication. This project has a documented history of an executing agent
   inventing commit SHAs and test output.** Every SHA you report comes from a real
   `git` command you ran. Every "tests pass / CI green" comes from a real run you
   can show. Every `file:line` you cite comes from a file you actually opened.
   Every external fact (a library's API, a license, a flag) comes from a page you
   actually fetched today — not memory. A negative claim ("there's no X", "this is
   dead code", "nothing reads this") gets an active disproof (a grep that returns
   nothing) before it is stated.

3. **Minimum change.** Smallest viable diff for the task. No drive-by refactors or
   "while I'm here" cleanups. Note adjacent issues for the maintainer; don't fix
   them in the same PR.

4. **Open a PR; never merge.** The maintainer (Ibrahim) is the only authorized
   actor for merges and any production/release action.

5. **CI must be green.** CI (`.github/workflows/ci.yml`) runs on **ubuntu, macOS,
   and Windows**: `cargo test --all`, `cargo clippy --all-targets --all-features
   -- -D warnings` (a single new warning fails CI), `cargo fmt --all -- --check`,
   and `cargo audit`. Run all four locally before marking a PR ready.

6. **Verify tech currency against today's date.** When a change depends on a
   third-party API/flag/behavior (tokio, yt-dlp, aria2, a crate), confirm it
   against the version in `Cargo.lock` and the current upstream docs — not
   training memory.

7. **Respect the invariants.** Before changing the download/extraction/queue
   paths, read [`docs/ai-os/invariants.md`](docs/ai-os/invariants.md). The
   timeout-bounding rule, the progress/command contract, the locking hierarchy,
   and the event-sourcing append-only log are load-bearing.

## Where things are

- The full system map: [`docs/ai-os/architecture.md`](docs/ai-os/architecture.md)
- Things that must stay true: [`docs/ai-os/invariants.md`](docs/ai-os/invariants.md)
- Open + completed work: [`docs/ai-os/backlog.md`](docs/ai-os/backlog.md)
- Where the project is now: [`docs/ai-os/status.md`](docs/ai-os/status.md)
- Key decisions: [`docs/ai-os/adr/`](docs/ai-os/adr/)

## Project commands

```
cargo build                 # debug build
cargo run                   # launch the GUI
cargo run -- <url> [flags]  # CLI mode (see src/cli.rs)
cargo test --all            # all tests
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo audit
```

## A note for prompt-building sessions

Two sister disciplines generate work prompts for this repo: a **read-only
investigate-first audit** (when the right shape is uncertain) and a
**system-modification** fix/feature prompt (when the shape is decided and state
is verified). Both assume this doc pack exists and is current — if you change an
invariant, the architecture, or the project state, update the relevant
`docs/ai-os/` file in the **same PR**.
