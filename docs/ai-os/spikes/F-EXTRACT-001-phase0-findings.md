# Spike Findings — F-EXTRACT-001 Phase-0 (proxy-capture, res-downloader style)

**Base:** origin/main `9eaee55fa331e0dd11d1944c9361164d71a1b284`
**Date:** 2026-07-02
**Status:** Desk-work COMPLETE · machine-level prototype **DEFERRED to a supervised
session** (see §6). Read-only: no `src/`, `Cargo.*`, or CI files touched; keychain
and system proxy never modified.

This spike answers the base-independent questions of the proxy-capture build plan
and produces a go/no-go recommendation. The security-sensitive proofs (root-CA
install, system-proxy rewrite, real-site pinning) are intentionally NOT run
unattended and NOT fabricated; they need the maintainer present for sudo + teardown
confirmation.

---

## 1. Base / working-tree divergence — RESOLVED

The local dev tree (`/Users/hanafi/rustloader-main`) is a **stale snapshot frozen
at `71c463c`** (PR #24, the initial doc-pack, 2026-07-01). Proof: local
`backlog.md` is **byte-identical** to `origin@71c463c:docs/ai-os/backlog.md` (both
84 lines). origin/main has since advanced to **`9eaee55`** through PRs #27–#37
(the download-reliability + history arc), growing `backlog.md` → 359 lines and
`status.md` → 257.

**Authoritative base = origin/main `9eaee55`.** `F-EXTRACT-001` lives at
`backlog.md:330` on origin (not `:61` as in the stale tree). All future prompts and
docs must be authored against a fresh origin clone; the local snapshot should be
refreshed (`git pull` / re-clone) so sessions don't start a day behind.

## 2. Crate / license verification (live, crates.io/lib.rs/docs.rs, 2026-07-02)

| Crate | Version | License | Verdict for rustloader (MIT, ADR 0001/0002) |
|---|---|---|---|
| **hudsucker** | 0.24.1 | MIT OR Apache-2.0 | ✅ **chosen base** — maintained, ~161k downloads, HTTP/2 + WS, rcgen-CA |
| **rcgen** | 0.14.8 | MIT OR Apache-2.0 | ✅ CA generation; ECDSA P-256 via `PKCS_ECDSA_P256_SHA256` |
| ideamans-hudsucker (fork) | 0.25.0 | MIT OR Apache-2.0 | ✅ compatible; adds `request_uri`/`request_method` to context — but low adoption (~274 downloads) |
| **slinger-mitm** | 0.0.5 | **GPL-3.0-only** | ❌ **REJECTED** — copyleft, violates ADR 0002 / I-9, MIT-incompatible (its base `slinger` is GPL too) |
| vsd | 0.5 | Apache-2.0 | separable — see §5 (stream-download half, non-DRM only) |
| res-downloader (reference) | 3.1.3 | Apache-2.0 | reference only; Apache-2.0 confirmed |

**Decision — MITM base = `hudsucker`.** slinger-mitm is eliminated on license alone
(GPL-3.0-only) before any feature comparison — the same copyleft trap ADR 0002 was
written for. This corrects the build plan, which had floated slinger-mitm as a
"strong alternative" without a verified license.

## 3. hudsucker request↔response correlation nuance

Stock `hudsucker`'s `HttpContext` carries only `client_addr: SocketAddr` — it does
**not** expose the request URI, so `handle_response(&HttpContext, Response)` cannot
see the originating URL directly. Media labelling/naming needs that URL. Options:
- (a) stash the URI in handler state during `handle_request` — racy under HTTP/2
  multiplexing on one connection;
- (b) `ideamans-hudsucker` adds `request_uri`/`request_method` to the context —
  clean, but low adoption argues against depending on the fork.

**Recommendation:** stock `hudsucker` + a robust per-stream correlation shim (keyed
by connection + stream id), with the fork's patch as a reference implementation.

## 4. Integration shape (sketch — honours ADR 0001/0002/0003, I-1/I-8/I-9)

A **default-OFF, clearly-experimental `capture` module** containing:
- a **CA manager** (rcgen, ECDSA P-256, private key `0600`, generated per-install,
  **never shipped/shared** — mitmproxy's rule);
- a **trust-store installer** (platform-specific; macOS `security add-trusted-cert`,
  etc.) — explicit user consent, no silent system-wide install;
- a **system-proxy manager** (set/restore), with teardown wired to normal-disable
  **and** process-exit **and** SIGINT (res-downloader's #1 support issue is a proxy
  left set → "no internet after closing").
- every `security`/`networksetup` shell-out is **timeout-bounded + killed on
  timeout** (I-1/ADR 0003).

Capture **only detects and lists** media (URL ext + Content-Type + `#EXTM3U`/`<MPD`
body sniff); the actual **download hands off** to rustloader's existing paths — the
native segmented engine for direct media, yt-dlp (now) / vsd (later, non-DRM) for
HLS/DASH. No third-party downloader is bundled (I-9); the Content-Type guard (I-8)
still governs what is written. Pinned domains **pass through untouched**.

## 5. vsd (stream-download half) — separable from this spike

vsd (Apache-2.0) was marked "not adopted" by the spike prompt's scope, but that
reflected an earlier stance. Per the maintainer's later decision, vsd is adopted for
the **HLS/DASH download half** (native Rust, cancellable, progress callbacks, subs +
ffmpeg mux), used **for non-DRM streams only** (never wire the Widevine/PlayReady
key-acquisition path; encrypted → skip). It composes with this capture layer:
`hudsucker` captures an `.m3u8`/`.mpd` URL → vsd downloads it. vsd's adoption is its
own investigate-first spike (dependency-tree/version-compat + ffmpeg-absent
handling) and is **not** launch-critical.

## 6. Preliminary go/no-go — GO (gated)

**Leaning GO** for a phased, default-OFF track: the crate foundation is license-clean
(`hudsucker` + `rcgen`) and the approach is proven (res-downloader). **Final go/no-go
is gated on a supervised prototype** proving the two things that can actually sink it:
1. **Guaranteed CA + system-proxy teardown** on normal / exit / crash paths.
2. **Cert-pinning passthrough** — pinned domains route untouched, not breaking the
   user's other apps.
Plus a **value check** (per the plan): on real target sites, how many are
capturable **and not already covered by yt-dlp** and **not pinned/DRM-only** — to
confirm the feature's marginal value justifies a root-CA feature.

## 7. Deferred to a SUPERVISED session (not run, not fabricated)

- MITM + media detection against a local test flow.
- Full CA lifecycle **including teardown** (needs sudo for the System keychain).
- Real-site cert-pinning behaviour.
These are hard-to-reverse, machine-level, security-sensitive actions; per the repo's
anti-fabrication rule (I-11) their "real command output" will only be produced with
the maintainer present.

## Sources
crates.io / lib.rs / docs.rs (hudsucker, ideamans-hudsucker, slinger-mitm, rcgen),
docs.rs/rcgen, docs.rs/hudsucker (HttpHandler/HttpContext), github.com/putyy/res-downloader,
github.com/clitic/vsd — all fetched 2026-07-02.
