# SPIKE probe — overlay strategy (a) diagnostics (self-grading)

**Throwaway. Never ship, never merge.** Lives only on branch
`spike/overlay-a-investigation`. Companion doc:
[`docs/spike-overlay-a-findings.md`](../../docs/spike-overlay-a-findings.md).

Standalone diagnostic extension. It never contacts the bridge, never sends
a download, holds no token, and does not touch the shipped Rustloader
Companion v0.3.0 (leave it installed; the probe only observes).
`media-filter.js` here is a byte-for-byte copy of
`extension/chrome/media-filter.js` at `a0a45ea`, so "the probe detected it"
≡ "the shipped sniffer would detect it".

The probe **grades itself**: for every detection it computes a mappability
verdict (`DIRECT` / `FRAME_SCOPED` / `TIMING_ONLY` / `UNMAPPABLE` — defined
at the top of `probe-worker.js`) and prints one summary line per site. You
copy that line; nobody eyeballs raw frame dumps unless a grade looks wrong.

## One-time setup (~2 minutes)

1. `git checkout spike/overlay-a-investigation` in `~/rustloader`.
2. Chrome → `chrome://extensions` → enable **Developer mode** (top right).
3. **Load unpacked** → select `~/rustloader/spike/overlay-a-probe/`.
4. On the probe's card, click the **service worker** link and keep that
   console window open — all output appears there.
5. Pin the probe's toolbar icon (puzzle-piece menu → pin).

## Per-site procedure (≈1 minute per site)

1. New tab → open the site → **play the video ~15 s** (interact with the
   embed itself if it's embedded).
2. Wait until ~30 s after page load (the probe scans the DOM until then).
3. With that tab active, click the **probe's toolbar icon**. Nothing happens
   in the page — look at the worker console.
4. Copy the **`SITE_VERDICT …` line** (and the `MUTOBS …` lines) and paste
   them back, labelled with the site.
5. Only if a grade surprises you (or I ask): also copy the per-detection
   lines and/or the full JSON block between the `DUMP` markers.

`"no record for this tab"` means the tab predates the probe — reload and
redo.

## The matrix

| # | Site | Why |
|---|------|-----|
| 1 | `youtube.com` — any normal watch page | The main target. Settles empirically whether YT's player is top-frame or iframe, and whether the sniffer detects anything there at all. |
| 2 | A plain `<video src=…mp4>` page (e.g. MDN's video example, or any simple page you know) | Baseline that SHOULD grade `DIRECT`. |
| 3 | `https://hlsjs.video-dev.org/demo/` | HLS via MSE: sniffed `.m3u8` vs `blob:` video src — the canonical mismatch; expected battleground for `TIMING_ONLY`. |
| 4 | Any blog/news page **embedding** a YouTube or Vimeo iframe | Cross-origin embed → the `FRAME_SCOPED` test. |
| 5 | A shadow-DOM player if you know one (e.g. Reddit) — **skippable**, say "skipped" | Shadow-root visibility check. |
| 6 | **The site you actually wanted this feature for — please name it** | The real acceptance test; everything else is proxy evidence. |
| 7 | One idle, media-free page (e.g. a Wikipedia article) — just load it, wait 30 s, dump | No `SITE_VERDICT` content expected; this run is for the `MUTOBS` lines: the measured MutationObserver rate that quantifies strategy (a)'s standing perf tax. |

**Early-exit rule:** if sites 1–3 already settle the question (e.g. YouTube
grades entirely `UNMAPPABLE`/`FRAME_SCOPED`), stop and paste what you have —
don't pad the matrix. Site 6 and 7 are worth running regardless (6 is the
point; 7 is one line of perf data).

## Cleanup when the spike is done

`chrome://extensions` → Remove the probe. The branch is deleted/archived
after the findings doc is finalised; nothing from `spike/` ever merges.
