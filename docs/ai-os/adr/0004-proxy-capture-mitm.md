# ADR 0004 — Proxy-capture via a MITM interception layer (hudsucker + rcgen)

**Status:** Proposed (draft — gated on the F-EXTRACT-001 Phase-0 *supervised*
prototype; not Accepted)
**Date:** 2026-07-02

## Context
`F-EXTRACT-001` asks for a media-capture capability competitive with tools like
`putyy/res-downloader`. The master capability audit (`612bda9`) confirmed rustloader
has **no proxy capability of any kind** today. "Proxy" has two unrelated meanings;
this ADR is about **proxy-CAPTURE / interception** (run a local MITM proxy, intercept
HTTPS from any app/browser, detect media, hand it to the downloader) — not
download-through-proxy (trivial, separate).

The reference tool, res-downloader (Apache-2.0, Go/Wails, 18k★), is a **system-proxy
MITM sniffer**: it installs a root CA, sets the system proxy, filters media out of
intercepted traffic, and **hands large downloads to external managers**. rustloader's
opportunity is to *own* the download half with its proven reliability arc (segmented,
206-guarded resume, sidecar identity, orphan cleanup).

Phase-0 desk research (see `docs/ai-os/spikes/F-EXTRACT-001-phase0-findings.md`)
verified the crate/license landscape on 2026-07-02:
- `hudsucker` 0.24.1 — MIT/Apache — the leading Rust MITM proxy.
- `rcgen` 0.14.8 — MIT/Apache — per-install CA, ECDSA P-256.
- `slinger-mitm` 0.0.5 — **GPL-3.0-only** — rejected (ADR 0002 / I-9).
- `res-downloader` — reference only (Apache-2.0).

Installing a root CA is the single most security-sensitive action a desktop app can
take: mishandling the CA key endangers **all** of the user's HTTPS. This feature is
larger and riskier than the entire download-reliability arc combined.

## Decision (proposed)
Implement proxy-capture as a **system-proxy MITM interception layer** built on
**`hudsucker`** (MIT/Apache) with a **per-install `rcgen` CA** (ECDSA P-256, private
key `0600`, generated on first enable, **never shipped or shared**). The feature is:
- **phased and default-OFF / experimental** until hardened;
- **capture-only** — it detects and lists media (URL ext + Content-Type + playlist
  body sniff) and **hands the download** to rustloader's existing paths (native
  segmented for direct media; yt-dlp now / vsd non-DRM later for HLS/DASH). No
  third-party downloader is bundled (ADR 0002 / I-9); the Content-Type guard (I-8)
  still governs writes;
- **teardown-guaranteed** — the CA trust and system-proxy changes are reversed on
  disable, process-exit, and SIGINT; all `security`/`networksetup` shell-outs are
  timeout-bounded and killed on timeout (ADR 0003 / I-1);
- **pinning-aware** — certificate-pinned domains pass through untouched;
- **no DRM circumvention** — the Widevine/PlayReady key-acquisition path (e.g. in
  vsd) is never wired; encrypted streams are skipped.

`slinger-mitm` is **rejected** on license (GPL-3.0-only) per ADR 0002, independent
of features.

## Consequences
- **Positive:** app-agnostic capture (any app, not just a browser) paired with
  native reliable downloading — a real differentiator; license-clean foundation.
- **Negative / risk:**
  - Root-CA install requires admin/elevation; antivirus/EDR may flag a
    CA-installing, system-proxy app; corporate machines may forbid it.
  - A teardown failure leaves the user without internet ("proxy left set") — the
    dominant support issue for tools of this class.
  - Certificate pinning defeats MITM on many high-value apps; DRM streams are out of
    scope — so the *marginal* value over yt-dlp's existing coverage may be narrower
    than it appears and must be justified (value gate, below).
  - Firefox's separate NSS trust store and per-OS quirks add cross-platform cost.
  - Legal/ToS: capturing streaming media touches site ToS and region-specific law; a
    user-responsibility disclaimer is required.
- **Neutral:** a substantial new dependency surface (`hudsucker`, `rcgen`, TLS
  stack); overlaps with the existing reqwest/tokio tree — compatibility to be
  confirmed in Phase-1.

## Alternatives considered
- **Browser-extension / CDP capture** — lighter, no root CA, but browser-tabs only;
  rejected for the app-agnostic desktop goal (kept as a possible fallback).
- **Download-through-proxy only** — trivial (`reqwest` + `yt-dlp --proxy`); does not
  meet the capture requirement; may still ship as a small separate feature.
- **`slinger-mitm` base** — rejected (GPL-3.0-only).
- **Do nothing / rely on yt-dlp coverage** — the null option the value gate must
  beat.

## Open gates before this ADR can move to Accepted
1. Supervised prototype proves **guaranteed CA + system-proxy teardown** (normal /
   exit / crash).
2. Supervised prototype proves **cert-pinning passthrough**.
3. **Value check:** on real target sites, enough are capturable AND not already
   covered by yt-dlp AND not pinned/DRM-only to justify a root-CA feature.
4. Security review of CA key storage + consent + uninstall.

## Proposed invariant (on acceptance)
Candidate `I-12`: the capture/MITM layer is default-OFF, generates a unique
per-install CA that is never shipped, and guarantees trust + system-proxy teardown on
every exit path. To be recorded in `invariants.md` if/when Accepted.
