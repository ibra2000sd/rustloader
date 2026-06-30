# ADR 0002 — Third-party binaries via external subprocess; no GPL bundling

**Status:** Accepted
**Date:** 2026-06-30

## Context
rustloader relies on external tools (today `yt-dlp`; possibly `aria2c` in future).
How a third-party tool is integrated has licensing consequences:
- **Arms-length subprocess** — spawn a separately-installed binary, located via
  bundle/PATH detection. The dominant view is this does not make rustloader a
  derivative work of the tool; no third-party code is linked or redistributed.
- **Bundling the binary** into the distribution (as the macOS `.app` bundles
  yt-dlp) **redistributes** that binary, which for a copyleft (GPL) tool triggers
  the GPL's obligations (source offer, license text, notices) **on the whole
  distribution** — incompatible with an MIT posture (`0001`).

Verified facts (2026-06-30): **aria2 is GPL-2.0+** (aria2.github.io; copyleft —
distributing binaries requires providing/offering source). **yt-dlp is The
Unlicense / public domain**, so bundling it imposes no copyleft — which is why the
existing yt-dlp bundling is unproblematic and does not generalize to aria2.

## Decision
Integrate all third-party download/extraction binaries as **external subprocesses**
located via detection (mirroring `find_ytdlp`). **Do not bundle GPL-licensed
binaries** (e.g. aria2) into the distribution. If aria2 is adopted (`F-DL-001`),
require/auto-detect an externally-installed `aria2c`; do not ship it inside the
app.

## Consequences
- Keeps the distribution's license clean (MIT, once `0001` lands).
- Users must have the external tool installed (same model as yt-dlp); detection +
  a clear "not found" message are required.
- Public-domain tools (yt-dlp) may still be bundled; this ADR constrains copyleft
  binaries specifically.

## Invariant
Recorded as `I-9` in `invariants.md`.
