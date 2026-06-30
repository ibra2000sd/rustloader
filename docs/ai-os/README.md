# docs/ai-os — the rustloader documentation pack

This is the maintained "operating system" for AI-assisted work on rustloader. It
exists so sessions don't re-derive (and mis-derive) the same context, and so the
same classes of mistake don't recur.

| File | What it is | Update when |
|------|------------|-------------|
| [`architecture.md`](architecture.md) | The verified system map: modules, the extraction and download pipelines, the actor/queue model, persistence. | The structure or a data path changes. |
| [`invariants.md`](invariants.md) | The things that must stay true across changes. The "don't break this" list. | A new invariant is established or an old one is intentionally changed (→ ADR). |
| [`backlog.md`](backlog.md) | Open and recently-closed work items, each with an ID, status, blast radius, and a bug-class description. | An item opens, changes status, or closes (close it in the same PR that does the work). |
| [`status.md`](status.md) | Where the project is right now: version, HEAD, current focus, what's done, what's next. | At the end of any session that lands work. |
| [`adr/`](adr/) | Architecture Decision Records — one file per decision, append-only. | A architecturally-significant decision is made. |

## Conventions

- **Ground truth over memory.** Every `file:line`, SHA, and external fact in these
  docs should be verifiable in the repo or a cited source. If you can't verify it,
  mark it `(unverified)` or leave it out.
- **State vs plan.** "Architecture" and "invariants" describe what IS (verified).
  "Backlog", "status", and ADRs may describe what's DECIDED or PLANNED — label
  those clearly so nobody mistakes a plan for a fact.
- **Backlog IDs.** `B-<AREA>-NNN` for bugs, `F-<AREA>-NNN` for features. Areas in
  use: `DL` (download engine), `EXTRACT` (extraction), `QUEUE`, `DOC`, `BUILD`,
  `GUI`.
