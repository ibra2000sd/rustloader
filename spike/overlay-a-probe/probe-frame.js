// ─── THROWAWAY SPIKE INSTRUMENTATION — NEVER SHIP, NEVER MERGE ───
// Branch: spike/overlay-a-investigation. Runs in EVERY frame
// (all_frames: true) and reports what THIS frame can see: its origin,
// whether it is the top frame, whether it can reach window.top, and a
// census of <video> elements (light DOM + open shadow roots — closed
// shadow roots are undetectable from a content script by design; if a
// player exists but never shows up in any frame's census, that absence is
// itself a finding).
//
// Sends a report at document_idle, then rescans every 3 s for ~30 s
// (players mount late) and re-sends only when something changed. After the
// 10th scan it goes silent — the probe has no standing observers, so its
// perf footprint is bounded and does NOT model strategy (a)'s permanent
// MutationObserver cost.

(() => {
  "use strict";
  if (window.__rustloaderProbeLoaded) return;
  window.__rustloaderProbeLoaded = true;

  const SCAN_EVERY_MS = 3000;
  const MAX_SCANS = 10;
  const NODE_BUDGET = 30000; // skip the shadow-root walk on giant documents

  function videoInfo(v, where) {
    const r = v.getBoundingClientRect();
    const cur = v.currentSrc || "";
    return {
      where, // "light-dom" | "open-shadow"
      srcAttr: v.getAttribute("src"),
      currentSrc: cur ? cur.slice(0, 200) : null,
      srcIsBlob: cur.startsWith("blob:"),
      sourceChildren: v.querySelectorAll("source").length,
      readyState: v.readyState,
      paused: v.paused,
      // NOTE: coordinates are frame-local. A top-frame overlay cannot use a
      // child frame's rect directly; it would also need the <iframe>
      // element's own offset in the parent.
      rect: {
        x: Math.round(r.x),
        y: Math.round(r.y),
        w: Math.round(r.width),
        h: Math.round(r.height),
      },
    };
  }

  function collectVideos() {
    const videos = [...document.querySelectorAll("video")].map((v) =>
      videoInfo(v, "light-dom"),
    );
    let shadowWalkSkipped = false;
    const nodeCount = document.querySelectorAll("*").length;
    if (nodeCount > NODE_BUDGET) {
      shadowWalkSkipped = true;
    } else {
      const walk = (root) => {
        for (const el of root.querySelectorAll("*")) {
          if (el.shadowRoot) {
            for (const v of el.shadowRoot.querySelectorAll("video")) {
              videos.push(videoInfo(v, "open-shadow"));
            }
            walk(el.shadowRoot);
          }
        }
      };
      try {
        walk(document);
      } catch {
        shadowWalkSkipped = true;
      }
    }
    return { videos, shadowWalkSkipped, nodeCount };
  }

  function canReachTop() {
    try {
      void window.top.location.href;
      return true;
    } catch {
      return false;
    }
  }

  let lastSent = "";
  let scans = 0;

  function report() {
    scans += 1;
    const { videos, shadowWalkSkipped, nodeCount } = collectVideos();
    const payload = {
      origin: location.origin,
      href: location.href.slice(0, 300),
      isTop: window === window.top,
      topAccessible: canReachTop(),
      fullscreenElement: document.fullscreenElement?.tagName ?? null,
      iframeCountInThisFrame: document.querySelectorAll("iframe").length,
      videoCount: videos.length,
      videos: videos.slice(0, 10),
      shadowWalkSkipped,
      nodeCount,
      scan: scans,
      t: Date.now(),
    };
    // Compare everything except the counters so unchanged pages go quiet.
    const wire = JSON.stringify({ ...payload, scan: 0, t: 0 });
    if (wire !== lastSent) {
      lastSent = wire;
      try {
        chrome.runtime
          .sendMessage({ type: "frame-report", report: payload })
          .catch(() => {});
      } catch {
        // Extension reloaded out from under the page; nothing to do.
      }
    }
    if (scans < MAX_SCANS) setTimeout(report, SCAN_EVERY_MS);
  }

  report();
})();
