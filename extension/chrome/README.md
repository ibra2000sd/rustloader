# Rustloader Companion (Chrome, Phase 2)

The browser half of rustloader's browser integration (F-EXT-001): a Manifest
V3 extension that detects media on pages (an observe-only sniffer), lists it
in a toolbar popup with a quality/format choice, adds a right-click
**"Download with Rustloader"** item to pages, links, and media elements, and
hands the URL plus the page's cookies to the Rustloader app over its loopback
bridge. Protocol and security model:
[`docs/browser-integration-design.md`](../../docs/browser-integration-design.md).

**License:** MIT, the same license as rustloader (see the repository root
`LICENSE`). Architectural patterns were studied from omniget (GPL-3.0) and
others, but no GPL code is included here.

## Install (developer / unpacked — no store listing yet)

1. Open `chrome://extensions`, switch on **Developer mode**.
2. **Load unpacked** → select this `extension/chrome/` directory.

## Pair it with the app

1. In Rustloader: **Settings → Browser Integration** → switch on
   **Browser integration (local bridge)**. (Press **Save Settings** so the
   toggle and token survive a restart.)
2. Automatic: click **Pair** in Rustloader's Settings, then within
   120 seconds open the extension's **Options** → **Pair automatically**.
3. Manual fallback: copy the token from Rustloader's Settings, paste it into
   Options → **Save**, then **Test connection**. Either way you should see
   "Paired with Rustloader … on port 4615x".

## Use it

With Rustloader running:

- **Toolbar popup** — when a page plays video/audio, the icon shows a per-tab
  count of detected streams (HLS/DASH manifests and direct video/audio,
  segment noise filtered out). Open the popup, pick a quality/format if you
  want one, and click **Download** on an item. **Pause capture on this tab**
  (bottom of the popup) stops detection for that tab and discards what was
  found there; reloading the page (or navigating, or closing the tab)
  re-enables it automatically. The pause is per-tab and in-memory only — a
  persistent per-site exclusion is a tracked follow-up.
- **Right-click** any page, link, or video/audio element →
  **Download with Rustloader**. The icon flashes **✓** when the app accepted
  the request or **!** when something failed — the Options page shows the
  last error.

## What it can access, and why

- `contextMenus` — the right-click item.
- `cookies` + `<all_urls>` — reading the *current page's* cookies so
  logged-in/age-gated content extracts correctly. Cookies are sent **only**
  to Rustloader at `127.0.0.1`, never anywhere else.
- `storage` — the pairing token and the cached bridge port
  (`storage.local`), and the per-tab detected-media list plus the per-tab
  capture-pause flag (`storage.session`, in-memory only, gone when the
  browser closes).
- `webRequest` (observational — MV3 extensions cannot block/modify) —
  seeing response content-types so media can be detected. Nothing is
  altered, redirected, or sent anywhere; matching URLs are kept per-tab and
  discarded when you navigate away or close the tab.
- `scripting` — used ONLY by the experimental, **off-by-default** in-page
  download button (below). While the option is off, no content script is
  registered and no extension code runs in any page.
- `http://127.0.0.1/*` — talking to the app's loopback bridge.

No page scripts unless you switch the experimental in-page button on; no
analytics either way.

## Experimental: in-page download button (off by default)

Options → **In-page download button** shows a small corner pill on pages
where the sniffer detected media (and nothing anywhere else). Clicking it
lists the same items as the popup; Download goes through the same local
bridge. The page-side script holds no token and reads nothing from the page
— it only renders what the extension's service worker sends it. Design and
threat model: [`docs/browser-integration-overlay-design.md`](../../docs/browser-integration-overlay-design.md).
This is a prototype pending a maintainer decision; it may be removed.

## Development

The sniffer's URL/content-type filtering, its capture-pause gate, and the
overlay's visibility decision are pure modules with node tests:

```
node --test extension/chrome/media-filter.test.js extension/chrome/capture-gate.test.js extension/chrome/overlay-logic.test.js
```

(`package.json` here only marks the directory as ES modules for node; Chrome
ignores it, and the release zip excludes both it and the test file.)

## Current limits

- The app must already be running (the extension tells you when it isn't).
- Chrome/Chromium only; Firefox and Edge packaging are later phases.
- Bridge-supplied cookies apply to extraction; see the design doc for the
  download-stage cookie note.
