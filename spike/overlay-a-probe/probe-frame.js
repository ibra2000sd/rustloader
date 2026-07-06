// ─── THROWAWAY SPIKE INSTRUMENTATION — NEVER SHIP, NEVER MERGE ───
// Branch: spike/overlay-a-investigation. Runs in EVERY frame
// (all_frames: true) and reports what THIS frame can see: its origin,
// whether it is the top frame, the MEASURED result of attempting a
// cross-origin read on window.top (the caught SecurityError is recorded
// verbatim, not inferred from origin strings), and a census of <video>
// elements (light DOM + open shadow roots — closed shadow roots are
// undetectable from a content script by design; if a player exists but
// never shows up in any frame's census, that absence is itself a finding).
//
// It also quantifies the standing perf tax strategy (a) would pay: a
// MutationObserver (childList+subtree+attributes+characterData on the
// whole document) runs for 30 s and counts callback invocations and
// mutation records, so "MutationObserver on every page forever" becomes a
// measured rate instead of a hand-wave. After 30 s the observer is
// disconnected and the script goes fully quiet — the probe itself has no
// permanent footprint.
//
// Reports at document_idle, then every 3 s until t≈30 s (players mount
// late); re-sends only when the payload changed.

(() => {
  "use strict";
  if (window.__rustloaderProbeLoaded) return;
  window.__rustloaderProbeLoaded = true;

  const SCAN_EVERY_MS = 3000;
  const MAX_SCANS = 11; // t = 0, 3, …, 30 s
  const NODE_BUDGET = 30000; // skip the shadow-root walk on giant documents

  // ── Measured, not inferred: can this frame read into window.top? ──
  // Per the HTML spec (CrossOriginPropertyFallback), reading a
  // non-allowlisted property such as location.href on a cross-origin
  // window throws a "SecurityError" DOMException. We attempt it and
  // record exactly what the browser did.
  function measureTopAccess() {
    try {
      void window.top.location.href;
      return { ok: true, error: null };
    } catch (e) {
      return { ok: false, error: `${e?.name ?? "Error"}: ${e?.message ?? e}` };
    }
  }

  // ── MutationObserver cost counter (30 s, then disconnected) ──
  const mutObs = { callbacks: 0, records: 0, startedAt: Date.now(), done: false };
  const observer = new MutationObserver((list) => {
    mutObs.callbacks += 1;
    mutObs.records += list.length;
  });
  try {
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
      characterData: true,
    });
  } catch {
    mutObs.done = true; // nothing to observe (e.g. XML doc quirk)
  }
  function mutObsSnapshot() {
    return {
      callbacks: mutObs.callbacks,
      records: mutObs.records,
      elapsedMs: Date.now() - mutObs.startedAt,
      done: mutObs.done,
    };
  }

  function videoInfo(v, where) {
    const r = v.getBoundingClientRect();
    const cur = v.currentSrc || "";
    const srcAttr = v.getAttribute("src");
    let srcResolved = null;
    if (srcAttr) {
      try {
        srcResolved = new URL(srcAttr, location.href).href;
      } catch {
        srcResolved = srcAttr;
      }
    }
    return {
      where, // "light-dom" | "open-shadow"
      srcAttr,
      srcResolved: srcResolved ? srcResolved.slice(0, 300) : null,
      currentSrc: cur ? cur.slice(0, 300) : null,
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

  let lastSent = "";
  let scans = 0;

  function report() {
    scans += 1;
    if (scans >= MAX_SCANS && !mutObs.done) {
      observer.disconnect();
      mutObs.done = true;
    }
    const { videos, shadowWalkSkipped, nodeCount } = collectVideos();
    const payload = {
      origin: location.origin,
      href: location.href.slice(0, 300),
      isTop: window === window.top,
      topAccess: measureTopAccess(),
      fullscreenElement: document.fullscreenElement?.tagName ?? null,
      iframeCountInThisFrame: document.querySelectorAll("iframe").length,
      videoCount: videos.length,
      videos: videos.slice(0, 10),
      shadowWalkSkipped,
      nodeCount,
      mutObs: mutObsSnapshot(),
      scan: scans,
      t: Date.now(),
    };
    // Compare without the counters/timestamps so unchanged pages go quiet,
    // but always send the final scan so the worker gets the 30 s MutObs
    // totals.
    const wire = JSON.stringify({ ...payload, mutObs: 0, scan: 0, t: 0 });
    if (wire !== lastSent || scans === MAX_SCANS) {
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
