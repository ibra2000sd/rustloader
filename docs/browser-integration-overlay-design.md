# In-page overlay ("floating download button") — design decision

Status: **proposed** — prototype gated OFF by default, awaiting maintainer verdict
Date: 2026-07-06 · Verified against `main` @ `f6dc559` · extension v0.2.1
Parent design: [`browser-integration-design.md`](browser-integration-design.md) (F-EXT-001)

This surface is **not** in the F-EXT-001 roadmap. Phase 2 (badge + popup,
shipped), Phase 3 (multi-browser + store), and Phase 4 (app-not-running) say
nothing about in-page UI. It is also the single largest threat-model and
store-review change the extension could make: its **first `content_scripts`
injection into arbitrary pages**. That is why this is a decision document
with a reversible prototype, not a committed direction.

---

## 1. Problem statement

Today the sniffer's detections surface in two places: the per-tab action
**badge count** and the toolbar **popup** list. Both live in the toolbar —
discoverable only if the user notices the toolbar icon (which Chrome hides
behind the puzzle-piece menu unless pinned). The motivating report: the
maintainer, using the extension on a real page, expected an IDM-style visual
cue **on the video itself** and initially concluded detection wasn't working.

Honest framing: this is an **N = 1 signal** from the maintainer's own
first-use session. It is consistent with how IDM/FDM train user expectations
("a button floats over the video"), but there is no broader evidence yet that
rustloader users want in-page UI, or that they'd tolerate an extension that
draws on every page. That uncertainty is exactly what the OFF-by-default
prototype is for.

## 2. What actually changes (the precise delta)

Manifest today (v0.2.1, verified at `f6dc559`):

```json
"permissions": ["contextMenus", "cookies", "storage", "webRequest"],
"host_permissions": ["http://127.0.0.1/*", "<all_urls>"]
```

No `content_scripts`, no `scripting`, no `web_accessible_resources`.

The delta is **not** a new host scope — `<all_urls>` is already granted (the
sniffer and cookie collection need it), so the install-time warning the user
sees does not materially change. The delta is:

1. **Code execution inside every page** (a content script), where today the
   extension only *observes* requests from the service worker. Observation
   cannot break a page; injection can.
2. One new API permission keyword, **`scripting`**, if registration is
   dynamic (§6). Per the current permission list (fetched from
   developer.chrome.com 2026-07-06), `scripting` carries **no install-time
   warning of its own** — the user-visible warning still comes from host
   permissions, already present.
3. **No `web_accessible_resources`.** The prototype inlines its icon as SVG
   inside the shadow root, so no extension resource is ever exposed to page
   origins. If a future version wants bitmap assets, the MV3 shape is
   `[{ "resources": [...], "matches": [...] }]` scoped to specific origins —
   but the right default is to keep this entry absent (it is a fingerprinting
   surface).

## 3. Threat-model delta

Baseline trust model (parent doc §6): the bridge defends the loopback port;
the extension holds a bearer token in `chrome.storage.local`; pages can't
read extension storage.

### What a content script in every page can and cannot do

A content script runs in an **isolated world**: "a private execution
environment that isn't accessible to the page or other extensions" —
JavaScript variables are invisible to page scripts and vice versa
(developer.chrome.com content-scripts concepts page, fetched 2026-07-06).
But the **DOM is shared**: the script reads and mutates the same document
the page owns. Concretely:

- It **can** read anything rendered in the page (text, form values in the
  DOM, media URLs) and mutate anything. This is why "we inject into every
  page" is a real trust statement to users and reviewers even if the script
  is 200 lines of overlay code — the *capability* is total DOM access.
- It **cannot** read the page's JS state, and the page cannot read the
  content script's variables. Isolation is bidirectional for JS.
- The isolated world is **not** a stealth or integrity boundary for DOM
  nodes: the page can see, restyle, move, or delete the overlay's host
  element. Shadow DOM (below) protects against *accidental* CSS collision,
  not against a hostile page. A hostile page can also **spoof** a
  convincing fake overlay; nothing in this design lets a fake overlay reach
  the token (see next point), so the worst case is UI confusion.
- Extension API surface inside a content script is limited to messaging and
  (if granted) `storage`. Everything else goes through the service worker.

### Token rule (non-negotiable)

The bridge token lives in `chrome.storage.local` and is used by
`bridge-client.js` inside the **service worker** (and the popup/options
pages, which are extension-origin documents). The content script must
**never** receive, hold, or request the token, and must never talk to
`127.0.0.1` itself. It sends `chrome.runtime.sendMessage` requests to the
worker ("what's the state for my tab?", "download item N"); the worker does
cookie collection and `sendDownload()` exactly as the context-menu and popup
paths do. Then even a fully compromised renderer/page that somehow subverted
the isolated world would hold no credential — it could at most ask the
worker to start a download of an already-detected URL for its own tab, which
is the same power any page already has by simply playing media in front of
the user.

Two supporting decisions:

- `chrome.storage.session` (the sniffer's per-tab detection store) stays at
  its default access level — **not exposed to content scripts** (the default
  per the storage API reference, fetched 2026-07-06). We do not call
  `setAccessLevel(TRUSTED_AND_UNTRUSTED_CONTEXTS)`; the worker brokers reads.
- `chrome.storage.local` (which holds the token) is never granted to the
  content script either — the overlay script does not use the `storage` API
  at all. All state arrives by message from the worker.

### What the overlay itself reads

Nothing. The prototype's content script reads **no page content** — it only
renders worker-supplied state (a count, a URL list the sniffer already had)
and forwards clicks. The DOM-read *capability* exists (that's the honest
threat-model delta), but the shipped code exercises none of it, which is
also the auditable claim for store review.

## 4. Store-review impact (Phase 3)

Fetched 2026-07-06:

- **Chrome Web Store — permissions policy**
  (developer.chrome.com/docs/webstore/program-policies/permissions):
  "Request access to the narrowest permissions necessary to implement your
  Product's features or services" … "If more than one permission could be
  used to implement a feature, you must request those with the least access
  to data or functionality" … don't "future proof" with unused permissions.
- **Chrome Web Store — review process**
  (developer.chrome.com/docs/webstore/review-process): "Reviews may take
  longer for extensions that request broad host permissions or sensitive
  execution permissions, or which include a lot of code or hard-to-review
  code." `<all_urls>` is called out explicitly as giving "extensive access
  to the user's web activity, especially when combined with other
  permissions."
- **Chrome Web Store — privacy tab**
  (developer.chrome.com/docs/webstore/cws-dashboard-privacy): each
  permission needs a written justification: "Fill out these fields to tell
  the reviewers why your extension needs to use each permission."
- **AMO (Firefox) policies**
  (extensionworkshop.com/documentation/publish/add-on-policies/): "Add-ons
  must only request those permissions that are necessary for them to
  function," and features that "make unexpected changes to web content"
  must be clearly disclosed and require **user opt-in**.

Reading for this feature:

- We already carry the heavy item (`<all_urls>` + `webRequest` + `cookies`);
  the extension is already in the slow-review bucket. Adding `scripting` +
  a content script **compounds** it: "sensitive execution permissions" and
  more code-injected-into-pages to review.
- The AMO clause is the sharpest constraint — an overlay drawn on pages is
  precisely an "unexpected change to web content," so AMO effectively
  *requires* the design we chose anyway: **disclosed, opt-in, default-off**.
  A default-ON overlay would be an AMO policy problem, not just a taste
  problem.
- Justification strings we'd submit: `scripting` — "renders an optional,
  user-enabled download button on pages where media was detected; injected
  only while the user has switched the option on"; the content script
  itself reads no page content (auditable in ~150 lines).
- Cost summary for Phase 3: longer review cycles on both stores, an extra
  privacy-tab justification, and a standing obligation that every future
  extension change is reviewed against "why do you inject into all pages."
  This cost is permanent once shipped to stores; that asymmetry is why the
  prototype stays store-unshipped until the kill/promote decision (§7).

## 5. Placement strategies (the fragility analysis)

The IDM experience people remember is a button anchored to the video. Three
ways to get something like it:

### (a) Anchor to detected `<video>` elements generically

Scan the DOM for `<video>` (MutationObserver for late-added players),
position an absolutely-placed button over each. **Honest costs:** misses
players inside cross-origin iframes (YouTube embeds — the top-frame script
can't see in; `allFrames: true` injection multiplies the footprint and still
needs frame⇄top coordination), misses closed shadow roots, misses canvas/
MSE-only players with no visible `<video>` in the light DOM, fights
full-screen and player-chrome z-index stacking, and needs a MutationObserver
running on **every page forever** — a standing perf tax on pages with no
media at all. Positioning over a moving/resizing player is a layout-tracking
problem (scroll, resize, fullscreen, PiP). Robust-ish on simple sites,
degrades unpredictably on exactly the sophisticated players people use most.

> **Update (2026-07-06): investigated empirically and rejected.** A
> read-only spike instrumented the sniffer's dropped frame fields and a
> per-frame `<video>` census on 5 real sites (13 detections): **zero**
> detections could be identity-mapped to a visible player — every observed
> player is MSE (`blob:` src ≠ detected manifest URL), and the
> maintainer's actual target sites add a cross-origin SecurityError
> (player in an embedded iframe). Best achievable grade was a fragile
> timing correlation, at a measured 9–12 MutationObserver callbacks/s.
> Full evidence and the do-not-relitigate verdict:
> [`spike-overlay-a-findings.md`](spike-overlay-a-findings.md). Strategy
> (b) is the deliberate ceiling.

### (b) One fixed-corner affordance per tab, shown only when the sniffer has detections

A single fixed-position pill in a screen corner, rendered only when the
existing network-level sniffer (which sees media the DOM approach can't —
HLS/DASH manifests, iframe-originated requests, MSE-fed streams) has ≥ 1
detection for this tab. **Honest costs:** it is *not* "the button on the
video" — it's a corner cue, IDM-lite; it can overlap site UI that also
likes corners (chat widgets, cookie banners); per-tab ≠ per-player (a page
with three videos gets one pill with a list, not three buttons). **What it
buys:** zero per-site DOM assumptions, no MutationObserver, no layout
tracking, ~150 lines, no behavior at all on pages without media, and it
reuses the sniffer as the single source of truth (no second detection
system to keep consistent).

### (c) Per-site adapters (YouTube, Vimeo, …)

Highest fidelity — exactly what IDM ships. **This is explicitly the
IDM-team cost:** a maintenance treadmill where every player redesign breaks
an adapter, multiplied per site, forever. IDM has a paid team for that
treadmill; rustloader has one maintainer. Also the worst store-review story
(most code, most DOM-touching). Not viable at this project's size; recorded
only so the trade-off is explicit.

### Recommendation: **(b)**, and (b) only

(b) tests the actual hypothesis — "does an in-page visual cue close the
discoverability gap?" — at the minimum threat/maintenance/review cost. If
(b) proves the hypothesis and users then ask for on-player buttons, (a) can
be layered later as a progressive enhancement *on top of* (b)'s plumbing
(the worker⇄content-script protocol is placement-agnostic). Choosing (a) or
(c) first would buy fidelity before we know the cue is wanted at all.
None of the three is free; (b)'s costs are the ones that don't compound.

## 6. Prototype mechanics (what PR-B implements)

- **Dynamic registration, not a static manifest entry.** The current Chrome
  docs recommend dynamic registration "when content scripts shouldn't
  always be injected on known hosts" (content-scripts concepts page,
  fetched 2026-07-06) — precisely the opt-in case.
  `chrome.scripting.registerContentScripts` (Chrome ≥ 96; requires the
  `scripting` permission) registers `overlay.js` for `http(s)://*/*` when
  `overlay_enabled` flips on; `unregisterContentScripts` removes it when it
  flips off. With the toggle off — the default — **no content script is
  registered at all**, which is materially better than a static all-pages
  script that self-disables: nothing is injected, parsed, or executed, and
  a reviewer can verify the gate in one place. Registrations persist across
  restarts (`persistAcrossSessions` defaults to `true`); the worker
  reconciles registration against the setting at startup in case they
  drift.
- **Show/hide contract:** overlay visible **iff** `overlay_enabled` ∧
  (detections for this tab > 0). Doubly enforced: the script isn't
  registered unless enabled, and the worker sends `visible: true` only when
  the count is positive. The decision function is a pure module
  (`overlay-logic.js`) with `node --test` coverage.
- **Rendering:** one host `<div>` at `document.documentElement` level with a
  **Shadow DOM** root (`mode: "open"` — a closed root adds no security
  against a hostile page and hampers debugging; the point is CSS isolation,
  which open shadow roots fully provide: page selectors don't pierce shadow
  boundaries, and inherited properties are reset at `:host`). All styles
  and the icon (inline SVG) live inside the root; `position: fixed` +
  maximal `z-index` on the host. Known limits, accepted for a prototype:
  the page can remove/restyle the host element itself, and top-layer page
  UI (`<dialog>`, fullscreen elements) draws above any z-index.
- **Data flow:** sniffer (untouched) keeps writing per-tab detections to
  `storage.session` → worker-side `overlay-host.js` observes
  `chrome.storage.session.onChanged` (every `StorageArea` has its own
  `onChanged`; storage API reference, fetched 2026-07-06) and pushes
  `{visible, count}` to the tab → click on the pill asks the worker for the
  detection list → "Download" on a row messages the worker, which runs the
  **same path as the popup/context menu**: `collectCookies` +
  `sendDownload` from `bridge-client.js`. No second send path, no token or
  `127.0.0.1` traffic in the content script, no quality/format UI (the
  popup keeps that; the overlay sends the default payload like the context
  menu does).
- **Footprint:** new files `overlay.js`, `overlay-host.js`,
  `overlay-logic.js` (+ test); existing files touched only for wiring
  (manifest: `scripting` + version bump; `background.js`: one import;
  options page: one checkbox). Bridge protocol, sniffer logic, token
  handling: zero changes.

## 7. Reversibility, rollout, kill criteria

- **OFF by default.** A fresh or upgraded install behaves exactly like
  v0.2.1 until the user ticks "Show in-page download button (experimental)"
  in options. No migration, no stored state unless enabled.
- **One-revert removal.** The prototype is a single PR whose only edits to
  existing files are the wiring lines above; `git revert <merge>` restores
  v0.2.1 behavior byte-for-byte and strands no state (an orphaned
  `overlay_enabled` key in `storage.local` is inert).
- **Not store-shipped.** The prototype exists for load-unpacked evaluation
  by the maintainer. The Phase-3 store submission does **not** include it
  unless it's been promoted (below).

**Promote to a real roadmap phase when:** the maintainer (and ideally ≥ 1
non-maintainer user) finds, over a few weeks of real use, that the corner
cue is the thing that makes detection discoverable — i.e., they stop
opening the popup via the toolbar and the overlay becomes the primary path
— and no recurring site-breakage reports appear. Promotion means: a
backlog phase of its own, store-listing justification text, and a decision
on strategy (a) as an enhancement.

**Kill (revert the PR) when any of:** the cue doesn't change usage (the
toolbar popup remains the path even with the toggle on); it visually
collides with real sites often enough to need per-site suppression rules
(that's the (c) treadmill arriving through the back door); store review for
Phase 3 flags the content script as a blocker for the whole extension —
the overlay is not worth delaying store presence of the working
badge+popup extension; or a security review finds any path from page JS
toward the bridge that the worker-broker design was supposed to preclude.

## 8. Open questions for the maintainer

1. Is the corner pill (strategy b) an acceptable answer to "IDM-style cue,"
   or is the on-player button (strategy a) the actual bar? If (a) is the
   bar, the honest answer may be "don't build this" — see §5(a)'s costs.
2. Cross-origin iframes: the top-frame overlay reports detections the
   sniffer attributes to the *tab*, including iframe media — is that
   good-enough coverage for the embed case (the pill shows; it just isn't
   inside the iframe)?
3. Should the toggle live in the extension options only (current plan), or
   also be surfaced from the popup footer where the user already looks?
