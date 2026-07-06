# SPIKE probe — overlay strategy (a) diagnostics

**Throwaway. Never ship, never merge.** Lives only on branch
`spike/overlay-a-investigation`. Companion doc:
[`docs/spike-overlay-a-findings.md`](../../docs/spike-overlay-a-findings.md).

This is a **standalone** diagnostic extension — it does not touch, replace,
or interfere with the shipped Rustloader Companion v0.3.0. You can leave the
real extension installed while running it (the probe observes; it never
downloads anything and never talks to the bridge). `media-filter.js` here is
a byte-for-byte copy of `extension/chrome/media-filter.js` at `a0a45ea`, so
"did the probe detect it" ≡ "would the shipped sniffer detect it".

## One-time setup (~2 minutes)

1. `git checkout spike/overlay-a-investigation` in `~/rustloader`.
2. Chrome → `chrome://extensions` → enable **Developer mode** (top right).
3. **Load unpacked** → select `~/rustloader/spike/overlay-a-probe/`.
4. On the probe's card, click the **service worker** link. A DevTools window
   opens on the worker's console. **Keep it open** — all dumps print there.

## Per-site procedure (repeat for each site in the matrix)

1. Open a **new tab**, navigate to the site.
2. **Play the video** for at least ~15 seconds (the probe rescans the DOM
   every 3 s for 30 s after load; playing forces the media requests the
   network side needs to see).
3. If the player is inside an embed, interact with it the way you normally
   would (click play on the embed itself).
4. With that tab still active, click the probe's **toolbar icon** (pin it
   from the puzzle-piece menu first if needed). Nothing visible happens in
   the page — the dump goes to the worker console.
5. In the worker console, copy everything between
   `──── RUSTLOADER SPIKE PROBE DUMP ────` and `──── END DUMP ────`
   (right-click the logged object area → usually easiest is to select the
   JSON text block itself) and paste it back to the session, labelled with
   the site name.
6. Optional but valuable: say in one line what you *saw* — where the video
   visually is, whether it was an embed, fullscreen behaviour if you tried
   it.

If a dump says `"no record for this tab"`, the tab predates the probe
install — reload the page and redo the steps.

## The site matrix (~6 sites)

| # | Site | Why it's in the matrix |
|---|------|------------------------|
| 1 | `youtube.com` — any normal watch page | The main target. Settles empirically whether YT's player is same-origin DOM or an iframe, and whether the sniffer detects anything on it at all. |
| 2 | A direct `<video src=…mp4>` page, e.g. `https://interactive-examples.mdn.mozilla.net/pages/tabbed/video.html` or any simple page you know with a plain mp4 | The easy case — baseline that SHOULD map cleanly. |
| 3 | `https://hlsjs.video-dev.org/demo/` (hls.js reference demo) | HLS through MSE: sniffed URL is a `.m3u8`, the visible `<video>` src is `blob:` — the canonical mismatch case. |
| 4 | Any blog/news page **embedding** a YouTube or Vimeo iframe (e.g. a WordPress post with an embedded YT video — pick any you know) | Cross-origin iframe: does the top frame see any `<video>`? Which frameId do detections land on? |
| 5 | A site with the player in shadow DOM if you know one (Reddit's `shreddit-player`, or skip if none handy) | Shadow-root visibility check. Skippable — say "skipped" if so. |
| 6 | **The site you actually wanted this feature for** — please name it | The real acceptance test. Everything else is proxy evidence. |

Early-exit rule: if after sites 1–3 the picture is already unambiguous
(e.g. YouTube detections land in frames no top-frame script can map), stop
and paste what you have — no need to pad the matrix.

## What the dump contains (so you know what you're pasting)

- `webNavigationFrames` — the tab's real frame tree (frameId, parent, URL).
- `frameEvents` — every sub-frame document load the network layer saw.
- `detections` — every media response, with the `frameId`/`frameType`/
  `initiator` fields the shipped sniffer currently discards, plus what the
  shipped media filter would have said about it.
- `frameReports` — per frame: origin, is-top, `<video>` census (src vs
  `blob:`, size, position), open-shadow-root findings.

Nothing here leaves your machine; the dump is console text you choose to
paste.

## Cleanup when the spike is done

`chrome://extensions` → Remove the probe. The branch is deleted or archived
after the findings doc is finalised; nothing from `spike/` ever merges.
