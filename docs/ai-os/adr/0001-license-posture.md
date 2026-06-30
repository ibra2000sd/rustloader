# ADR 0001 — License posture

**Status:** Accepted
**Date:** 2026-06-30 (accepted 2026-07-01)

## Context
The README advertises an MIT license (badge + "see the LICENSE file"), but no
`LICENSE`/`COPYING` file exists in the repo and `Cargo.toml` has no `license`
field. The stated license is therefore unbacked. This must be resolved before any
decision to depend on a copyleft (GPL) third-party binary such as aria2, because
the license of the distribution determines what that dependency obligates.

## Decision
Adopted **MIT**, matching the existing README claim (this is the maintainer's call;
it is his code). Concretely: added a real `LICENSE` file with the MIT text,
copyright holder "Ibrahim (ibra2000sd)", year 2026, and added `license = "MIT"`
to `Cargo.toml`. Tracked as backlog `B-DOC-001`.

## Consequences
- The README's claim becomes true; downstream users get a clear, permissive
  license.
- Any GPL-licensed component (e.g. aria2) may then only be used at arms-length
  (external subprocess), never bundled/redistributed under the MIT umbrella — see
  `0002`.

## Resolved
- MIT confirmed as the license. `LICENSE` and `Cargo.toml` `license = "MIT"`
  landed in the PR enacting this ADR (`B-DOC-001`).
