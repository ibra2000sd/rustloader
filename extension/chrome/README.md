# Rustloader Companion (Chrome, Phase 1)

The browser half of rustloader's browser integration (F-EXT-001): a Manifest
V3 extension that adds a right-click **"Download with Rustloader"** item to
pages and links, and hands the URL plus the page's cookies to the Rustloader
app over its loopback bridge. Protocol and security model:
[`docs/browser-integration-design.md`](../../docs/browser-integration-design.md).

**License:** MIT, the same license as rustloader (see the repository root
`LICENSE`). Architectural patterns were studied from omniget (GPL-3.0) and
others, but no GPL code is included here.

## Install (developer / unpacked — Phase 1 has no store listing)

1. Open `chrome://extensions`, switch on **Developer mode**.
2. **Load unpacked** → select this `extension/chrome/` directory.

## Pair it with the app

1. In Rustloader: **Settings → Browser Integration** → switch on
   **Browser integration (local bridge)** — a pairing token appears. Copy it.
   (Press **Save Settings** so the toggle and token survive a restart.)
2. In Chrome: right-click the extension → **Options** → paste the token →
   **Save**, then **Test connection**. You should see
   "Paired with Rustloader … on port 4615x".

## Use it

With Rustloader running: right-click any page (or a link) →
**Download with Rustloader**. The toolbar icon flashes **✓** when the app
accepted the request (the download then appears in Rustloader's queue) or
**!** when something failed — the Options page shows the last error.

## What it can access, and why

- `contextMenus` — the right-click item.
- `cookies` + `<all_urls>` — reading the *current page's* cookies so
  logged-in/age-gated content extracts correctly. Cookies are sent **only**
  to Rustloader at `127.0.0.1`, never anywhere else.
- `storage` — the pairing token and the cached bridge port.
- `http://127.0.0.1/*` — talking to the app's loopback bridge.

No webRequest/network sniffing, no page scripts, no analytics. Media
detection is a later phase.

## Phase-1 limits

- The app must already be running (the extension tells you when it isn't).
- Chrome/Chromium only; Firefox and Edge packaging are later phases.
- Bridge-supplied cookies apply to extraction; see the design doc for the
  download-stage cookie note.
