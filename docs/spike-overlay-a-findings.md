# SPIKE findings — player-anchored overlay (strategy a) is not viable

Status: **CLOSED — verdict recorded 2026-07-06. Strategy (a) rejected;
the corner pill (strategy b, shipped in extension v0.3.0) is the ceiling.**
Investigated against `main` @ `a0a45ea` (extension v0.3.0).
Probe code: branch `spike/overlay-a-investigation` @ `bd17462` (throwaway;
branch deleted after this doc merged — the SHA is recorded here for
archaeology only, nothing from it ships).
Parent design: [`browser-integration-overlay-design.md`](browser-integration-overlay-design.md) §5(a)

**The question this spike answered:** can a sniffed network URL be reliably
associated with the visible video player on real sites — or does the
sniffed-URL ↔ visible-`<video>` mismatch make an IDM-style over-the-player
button infeasible?

**The answer, from real data: infeasible.** Across 5 real site samples and
13 sniffer detections, not one detection could be identity-mapped to a
visible `<video>` (0 × DIRECT). Every HLS/DASH player observed uses Media
Source Extensions, so the visible `<video>.src` is always a `blob:` URL
that never equals the detected manifest URL; embedded players add a
cross-origin SecurityError on top. This is the browser's security model
plus MSE — not a limitation of rustloader's code, and no amount of
engineering on our side changes either. **This document exists so the
decision is not re-litigated.**

Evidence discipline: every claim is tagged **[code]** (verified in this
repo at `a0a45ea`, file:line cited), **[docs]** (fetched from the cited
page on 2026-07-06), **[observed]** (Ibrahim ran the probe in Chrome and
reported the output), or **[inference]**.

---

## 1. Ground truth established before any browser run

- **[code]** The shipped sniffer records only `{url, kind, detectedAt}` per
  detection (`extension/chrome/sniffer.js:82-86`); every frame field is
  dropped.
- **[docs]** Chrome's `webRequest.onHeadersReceived` details object DOES
  expose `frameId`, `parentFrameId`, `frameType`, `documentId` and
  `initiator` (developer.chrome.com/docs/extensions/reference/api/webRequest,
  fetched 2026-07-06; `frameType`/`documentId` current names, Chrome ≥ 106).
  So the frame plumbing strategy (a) would need was *available* — the spike
  tested whether it would help. (It doesn't: see §4.)
- **[docs]** Cross-origin reachability is measurable: reading a
  non-allowlisted property such as `location.href` on a cross-origin
  `window.top` throws a "SecurityError" DOMException (MDN same-origin
  policy page + html.spec.whatwg.org cross-origin property rules, both
  fetched 2026-07-06). The probe attempted the read in every frame and
  recorded the caught error, rather than inferring from origin strings.
- **[code]** The media filter deliberately keeps *manifests* and drops
  segment traffic (`extension/chrome/media-filter.js:69-87`). So on any
  MSE/HLS site, the URL available for anchoring is a manifest URL that no
  `<video>` element's `src` will ever equal — the structural mismatch the
  data then confirmed everywhere.
- **[code]** The shipped overlay content script is registered
  top-frame-only (`extension/chrome/overlay-host.js:40-49`).

## 2. Instrumentation (throwaway, self-grading)

A standalone diagnostic extension (`spike/overlay-a-probe/` on the spike
branch only — never merged; verified absent from `main`'s history). Worker
side: recorded every media-classified response with the frame fields the
sniffer drops, using a byte-for-byte copy of `media-filter.js` so probe
detections ≡ shipped-sniffer detections. Page side (`all_frames: true`):
per-frame `<video>` census (`currentSrc`/resolved `src` vs `blob:`,
geometry, open shadow roots), the measured `window.top` access result, and
a 30-second MutationObserver counter quantifying the standing perf tax.

At dump time the probe joined each detection to its frame's census by
`frameId` and emitted a machine-decided grade — no human or AI
interpretation between data and verdict:

- **DIRECT** — detected URL equals a `<video>` src in a top/reachable
  frame → anchorable.
- **FRAME_SCOPED** — detection's frame has a `<video>` but its measured
  `window.top` read threw → anchorable only to the `<iframe>` box.
- **TIMING_ONLY** — reachable frame with `<video>` but `blob:`/MSE src, no
  URL identity → correlation-only, fragile.
- **UNMAPPABLE** — no `<video>` in the detection's frame → corner pill is
  the only option.

## 3. Observed results — **[observed]**

Ibrahim ran the probe on 5 site samples (13 sniffer-positive detections
total) and reported the emitted verdicts. The raw console dumps were
reviewed in-session and are condensed below; they were not archived into
this doc. Matrix slots not run (early-exit rule, invoked once the picture
was unambiguous): the plain-`<video src=mp4>` baseline, a shadow-DOM
player, and the idle-page MutationObserver baseline — consequences in §6
and the scope-limit note in §5.

Per-site, as reported:

- **topcinemaa.top** (streaming site Ibrahim actually uses — the real
  target for this feature): player in a cross-origin iframe; top access
  threw SecurityError; video src `blob:` (MSE). Grade: **FRAME_SCOPED**.
- **elif.news** (2 pages; same real-use category): cross-origin iframe
  player, SecurityError, `blob:` src. Grade: **FRAME_SCOPED** (both).
- **hlsjs.video-dev.org/demo/**: top-frame player, reachable, but src is
  `blob:` (MSE) — the sniffed `.m3u8` has no URL identity with the
  element. Grade: **TIMING_ONLY**.
- **youtube.com/watch**: top-frame `blob:` player — and notably the
  sniffer's detections were only UI sound cues (`.mp3`), **not** the
  `googlevideo.com` media stream itself. Grade: **TIMING_ONLY**, on
  detections that aren't even the video.
- MutationObserver rate on these video pages: **9–12 callbacks/s** while
  the DOM churned (no idle baseline captured; see §6).

## 4. The truth table

| Site | Type | Sniffer detected? | Machine grade | Root cause |
|---|---|---|---|---|
| topcinemaa.top | streaming / cross-origin iframe | yes | FRAME_SCOPED | SecurityError + blob MSE |
| elif.news (page 1) | streaming / cross-origin iframe | yes | FRAME_SCOPED | SecurityError + blob MSE |
| elif.news (page 2) | streaming / cross-origin iframe | yes | FRAME_SCOPED | SecurityError + blob MSE |
| hlsjs.video-dev.org demo | direct HLS, top frame | yes | TIMING_ONLY | blob MSE — no URL identity |
| youtube.com/watch | large video, top frame | yes — but only UI `.mp3` cues, not the googlevideo stream | TIMING_ONLY | blob MSE; the anchorable detections aren't the video |

**DIRECT count across all 13 detections: 0.**

## 5. Verdict — **(a) is infeasible for the sites that matter**

Of the three pre-agreed outcomes, the data lands on the third:

> Strategy (a) — a download button anchored to the actual player — cannot
> be built honestly on what the sniffer detects. The sniffer detects
> manifests; MSE players expose only `blob:` srcs; identity mapping is
> therefore structurally absent (0 DIRECT). On Ibrahim's real target
> sites the player additionally lives in a cross-origin iframe
> (SecurityError on top access), capping the best case at "a button on
> the iframe box" — which is not the IDM experience that motivated (a).
> The best achievable association anywhere was TIMING_ONLY: a heuristic
> guess that degrades with multiple players, preloading, and SPA
> navigation. Building a feature whose *best* case is a fragile guess,
> at the standing cost in §6, is not justified. **The corner pill
> (strategy b), shipped OFF-by-default in extension v0.3.0, is the
> deliberate technical ceiling — not an unfinished feature.**

Honest scope limit, recorded so the boundary of the claim is explicit:
all 5 samples were HLS/blob players. A legacy page with a literal
`<video src="file.mp4">` would likely grade DIRECT — but that case is
rare on the sites this feature was requested for, and largely outside
what the manifest-targeting sniffer detects anyway
(`media-filter.js` keeps manifests and whole files; plain-file pages are
already well served by the context menu and corner pill). If someone
proposes (a) again, the burden is to show a *population of real sites*
that grades DIRECT — not to re-run this spike on the same MSE sites.

What would have to change for (a) to become viable: browsers exposing a
DOM↔network identity for MSE streams (no sign of this), or per-site
adapters (strategy c — explicitly rejected in the design doc as the
IDM-team treadmill).

## 6. Measured cost note

**[observed]** MutationObserver (childList+subtree+attributes+
characterData, whole document) fired at **9–12 callbacks/s** on the video
pages sampled. **[inference]** A real (a) would pay that observer on
*every* page for the *whole* page lifetime — plus one content-script
instance per iframe (`allFrames: true`, ad iframes included) plus
scroll/resize/fullscreen re-anchoring — versus the shipped (b), whose
content script does nothing until the worker pushes state. The idle-page
baseline was not captured (early exit); 9–12/s on active pages is already
sufficient to show the tax is real and continuous, but the idle number is
unknown.

## 7. Cleanup

- `spike/` never existed on `main` (verified: `git log main -- spike/` is
  empty).
- The spike branch is deleted after this doc merges. Proposed commands
  (Ibrahim runs them; the executing session does not delete branches):
  `git branch -D spike/overlay-a-investigation && git push origin --delete spike/overlay-a-investigation`
- This findings doc is the only survivor of the spike.

## 8. The open questions, answered

1. *Which site did Ibrahim actually want this for?* — the streaming sites
   sampled (topcinemaa.top, elif.news): **[observed]** both grade
   FRAME_SCOPED. The touchstone case fails; viability elsewhere is moot.
2. *Does YouTube's top-frame player frame-scope-map?* — moot at a deeper
   level than expected: **[observed]** the sniffer's YouTube detections
   were UI `.mp3` cues, not the stream. Anchoring them to the player would
   attach a download button to the wrong content.
3. *Fullscreen behavior?* — never reached; the mapping failed before
   positioning ever mattered.
