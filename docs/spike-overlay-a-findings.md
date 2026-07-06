# SPIKE findings — is player-anchored overlay (strategy a) viable?

Status: **Phase 1 pending — awaiting Ibrahim's probe runs.**
Date: 2026-07-06 · Investigated against `main` @ `a0a45ea` (extension v0.3.0)
Branch: `spike/overlay-a-investigation` (never merges; deleted/archived when done)
Probe: [`spike/overlay-a-probe/`](../spike/overlay-a-probe/README.md)
Parent design: [`browser-integration-overlay-design.md`](browser-integration-overlay-design.md) §5(a)

**The one question:** can a sniffed network URL be reliably associated with
the visible video player on real sites — or does the sniffed-URL ↔ visible-
`<video>` mismatch make strategy (a) infeasible, leaving the shipped corner
pill (strategy b) as the ceiling?

Evidence discipline used throughout: every claim is tagged
**[code]** (verified in this repo at `a0a45ea`, file:line cited),
**[docs]** (fetched from the cited page on 2026-07-06),
**[observed]** (Ibrahim ran the probe in Chrome and pasted the dump), or
**[inference]** (reasoned, not directly observed — could be wrong).

---

## 1. What was already true before any browser run

- **[code]** The shipped sniffer records only `{url, kind, detectedAt}` per
  detection (`extension/chrome/sniffer.js:82-86`). It has no notion of which
  frame a detection came from.
- **[docs]** Chrome's `webRequest.onHeadersReceived` details object DOES
  expose `frameId`, `parentFrameId`, `frameType`, `documentId` (optional),
  and `initiator` (developer.chrome.com/docs/extensions/reference/api/webRequest,
  fetched 2026-07-06, re-verified same day). Per the docs: frameId "0
  indicates that the request happens in the main frame; a positive value
  indicates the ID of a subframe in which the request happens." `frameType`
  and `documentId` are the current names, both added in Chrome 106 — the
  probe records `null` where absent. So the frameId plumbing strategy (a)
  would need is *available*; today's sniffer simply drops it.
- **[docs]** Cross-origin frame access is measurable, not guessable: per
  MDN's same-origin-policy page (fetched 2026-07-06), a cross-origin
  `window.top` is itself readable, but `location.href` on it is write-only;
  per the HTML spec's cross-origin property rules
  (html.spec.whatwg.org/multipage/nav-history-apis.html, fetched
  2026-07-06), reading a non-allowlisted property throws a
  **"SecurityError" DOMException** (`CrossOriginPropertyFallback`). The
  probe therefore *attempts* `window.top.location.href` in every frame and
  records the caught error verbatim, rather than inferring reachability
  from origin strings.
- **[docs]** `webNavigation.getAllFrames({tabId})` returns the full frame
  tree (frameId, parentFrameId, url, documentId, frameType) but requires the
  `webNavigation` permission the shipped extension does not carry
  (developer.chrome.com/docs/extensions/reference/api/webNavigation, fetched
  2026-07-06; `extension/chrome/manifest.json:12` — permission list verified
  at `a0a45ea`).
- **[code]** The shipped overlay content script is registered top-frame-only
  (`extension/chrome/overlay-host.js:40-49`: dynamic registration keeps the
  `allFrames` default, i.e. `false`).
- **[code]** The media filter deliberately drops segment traffic (`.ts`,
  `.m4s`, `video/mp2t`, `init/seg/chunk` basenames) and keeps manifests and
  whole files (`extension/chrome/media-filter.js:69-87`). So on an MSE/HLS
  site the detection to anchor is the *manifest* URL, which no `<video>`
  element's `src` will ever equal.
- **[inference]** Even with `allFrames: true` injection, a child frame's
  `getBoundingClientRect()` is frame-local; a top-frame overlay positioned
  over an iframe player needs the child rect *plus* the `<iframe>` element's
  own offset in the parent — i.e. cross-frame coordinate composition via the
  worker. Alternatively the top frame could anchor to the `<iframe>` element
  itself (which it can see and measure), sidestepping the composition but
  anchoring to the embed box rather than the player chrome. Neither variant
  is tested yet; Phase 1 data decides whether either is worth building.

## 2. Instrumentation added (throwaway)

A standalone diagnostic extension in `spike/overlay-a-probe/` (this branch
only). Worker side: logs every sub-frame document response and every
media-classified response **with the frame fields the sniffer drops**;
`media-filter.js` is a verbatim copy from `a0a45ea` so probe detections ≡
shipped-sniffer detections. Page side: an `all_frames: true` census script
reporting, per frame: origin, is-top, the **measured** result of a
cross-origin `window.top.location.href` read (caught SecurityError recorded
verbatim), `<video>` elements (`currentSrc`/resolved `src` vs `blob:`,
geometry, open-shadow placement), and a 30-second MutationObserver counter
(callbacks + mutation records on the full document) that turns the
"standing perf tax" claim into a measured rate. Join key: `sender.frameId`
(DOM side) ↔ `details.frameId` (network side).

**The probe grades itself.** At dump time (toolbar click) the worker joins
each sniffer-positive detection to its frame's census and emits a
machine-decided grade — `DIRECT` / `FRAME_SCOPED` / `TIMING_ONLY` /
`UNMAPPABLE`, defined precisely in the header of
`spike/overlay-a-probe/probe-worker.js` — plus one copyable summary line
per site (`SITE_VERDICT <host>: {…}`) and `MUTOBS` perf lines. The human
step is "copy the verdict line", not "interpret raw frames"; the raw JSON
is still dumped for auditing surprising grades.

The probe stops scanning (and disconnects its MutationObserver) ~30 s after
page load, so its own footprint is bounded; the `MUTOBS` rate it reports is
the measured cost a *permanent* observer would keep paying (§6).

## 3. Ibrahim's per-site observations — **[observed]**

> _Pending. Paste each site's `SITE_VERDICT` + `MUTOBS` lines verbatim
> under its heading (full JSON only where a grade looked wrong), plus the
> one-line "what I visually saw" note._

### 3.1 YouTube watch page
_pending_

### 3.2 Direct `<video src=…mp4>` page
_pending_

### 3.3 hls.js demo (MSE/HLS)
_pending_

### 3.4 Page embedding a YouTube/Vimeo iframe
_pending_

### 3.5 Shadow-DOM player (or "skipped")
_pending_

### 3.6 Ibrahim's actual target site (name: ___ )
_pending_

### 3.7 Idle media-free page (MutationObserver cost baseline)
_pending — `MUTOBS` lines only_

## 4. The truth table — the deliverable

| Site | Sniffer detected a downloadable URL? | `<video>` visible — which frame? | Detection ↔ player mappable? (by what signal) | Button positionable over the player? |
|---|---|---|---|---|
| YouTube watch page | _pending_ | _pending_ | _pending_ | _pending_ |
| Direct mp4 page | _pending_ | _pending_ | _pending_ | _pending_ |
| hls.js demo | _pending_ | _pending_ | _pending_ | _pending_ |
| YT/Vimeo embed page | _pending_ | _pending_ | _pending_ | _pending_ |
| Shadow-DOM player | _pending_ | _pending_ | _pending_ | _pending_ |
| Ibrahim's target site | _pending_ | _pending_ | _pending_ | _pending_ |

Columns 2–4 are filled directly from each site's machine-emitted
`SITE_VERDICT` line — the probe computes the grades; no human or AI
interpretation sits between the data and the table. The grades (decided by
`gradeDetection()` in `spike/overlay-a-probe/probe-worker.js`, definitions
in its header):
- **DIRECT** — detected URL equals a `<video>` `currentSrc`/resolved `src`
  in the same frame, and that frame is top or same-origin-reachable →
  anchorable.
- **FRAME_SCOPED** — the detection's frame has a `<video>` but its measured
  `window.top` read threw (cross-origin child) → anchorable only to the
  `<iframe>` box, not the player.
- **TIMING_ONLY** — reachable frame, `<video>` present, but src is
  `blob:`/MSE so no URL identity → association rests on timing/frameId
  correlation only → fragile.
- **UNMAPPABLE** — detection's frame shows no `<video>` at all (or no
  census arrived) → corner pill is the only option.

## 5. Verdict — one of three, evidence first

_Pending Phase 1. The three mutually exclusive outcomes and what each
triggers:_

- **(a) broadly viable** → follow-up spec: sniffer records
  `frameId`/`frameType`/`initiator` (additive fields on the existing
  detection record — popup/badge/overlay-b readers ignore them);
  `allFrames: true` registration footprint; worker-brokered frame⇄top
  coordinate composition; the mapping mechanism per the table's grades.
- **(a) works only for the direct-`<video>` case** → hybrid: anchored button
  when the mapping grade is **direct** or **frame-scoped** with a
  single-player frame; shipped corner pill otherwise. Spec the "is this
  mappable" decision function (pure module, testable, same style as
  `overlay-logic.js`).
- **(a) infeasible for the sites that matter (esp. YouTube / Ibrahim's
  target)** → stop at the corner pill, tag v0.11.0 with (b), and fold this
  doc's evidence into `browser-integration-overlay-design.md` §5(a) so the
  question isn't re-litigated from vibes next time.

## 6. Honest cost note (standing tax of a real strategy-(a) build)

_Measured numbers land here from the `MUTOBS` lines (site 7 gives the
idle-page baseline; sites 1–6 give real-player rates)._ Structurally
certain even before data: **[inference]** a real (a) needs (1) a
MutationObserver on every page for the whole page lifetime (the probe
disconnects at 30 s; the reported rate is what a permanent observer would
keep paying), (2) `allFrames: true` injection — one content-script instance
per iframe on every page, ad iframes included (the per-site
`framesWithVideo=M/F` denominators show how many instances that is on real
pages), and (3) scroll/resize/fullscreen re-anchoring listeners, which the
probe does not model at all. None of this exists in the shipped (b), whose
content script does nothing until the worker pushes state.

## 7. What to delete / keep

- Delete: `spike/overlay-a-probe/` (and the branch) once this doc's verdict
  is recorded. Nothing from the branch merges.
- Keep: this findings file — content to be folded into
  `browser-integration-overlay-design.md` (or linked from it) in a normal
  docs PR from a clean branch, per the verdict.

## 8. Open questions

1. Which site did Ibrahim actually want the on-player button for? (Matrix
   slot 6 — the real acceptance test. Asked, awaiting answer.)
2. If YouTube's player turns out same-origin/top-frame (plausible for
   youtube.com itself, unlike embeds), does its blob-src MSE player still
   frame-scope-map when the watch page preloads *other* videos' streams?
3. Fullscreen: `document.fullscreenElement` promotes the player into the
   top layer, above any z-index. Does the anchored button need to enter the
   fullscreen element's subtree to stay visible (a real DOM intrusion), or
   is "button disappears in fullscreen" acceptable?
