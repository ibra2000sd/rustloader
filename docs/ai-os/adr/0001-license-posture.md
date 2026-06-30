# ADR 0001 — License posture

**Status:** Proposed (awaiting maintainer confirmation)
**Date:** 2026-06-30

## Context
The README advertises an MIT license (badge + "see the LICENSE file"), but no
`LICENSE`/`COPYING` file exists in the repo and `Cargo.toml` has no `license`
field. The stated license is therefore unbacked. This must be resolved before any
decision to depend on a copyleft (GPL) third-party binary such as aria2, because
the license of the distribution determines what that dependency obligates.

## Decision (proposed)
Adopt **MIT**, matching the existing README claim (this is the maintainer's call;
it is his code). Concretely: add a real `LICENSE` file with the MIT text and the
correct copyright holder/year, and add `license = "MIT"` to `Cargo.toml`. Track as
backlog `B-DOC-001`.

## Consequences
- The README's claim becomes true; downstream users get a clear, permissive
  license.
- Any GPL-licensed component (e.g. aria2) may then only be used at arms-length
  (external subprocess), never bundled/redistributed under the MIT umbrella — see
  `0002`.

## Open
- Confirm MIT vs another license. If a different license is chosen, revise this
  ADR and `0002`'s assumptions accordingly.
