// ─── THROWAWAY SPIKE INSTRUMENTATION — NEVER SHIP, NEVER MERGE ───
// Branch: spike/overlay-a-investigation. Purpose: gather the per-frame
// evidence docs/spike-overlay-a-findings.md needs (can a sniffed URL be
// associated with the visible <video> player?) and GRADE it on the spot,
// so the human step is "copy the SITE_VERDICT line", not "interpret raw
// frames". Standalone diagnostic extension — the shipped Rustloader
// Companion (extension/chrome/) is untouched; no bridge, no downloads,
// no token.
//
// What it records, per tab, in chrome.storage.session:
//   frameEvents  — every sub_frame document response: the frame tree as the
//                  network layer sees it (frameId, parentFrameId, url).
//   detections   — every response media-filter.js classifies as media, PLUS
//                  any response Chrome itself types as "media" even when the
//                  filter says no. Each entry keeps frameId / frameType /
//                  documentId / initiator / resourceType — exactly the
//                  fields today's sniffer (extension/chrome/sniffer.js)
//                  throws away. (frameType/documentId exist in Chrome ≥ 106;
//                  recorded as null on older Chrome.)
//   frameReports — DOM census messages from probe-frame.js (all_frames):
//                  per frame, the <video> elements that frame can see,
//                  whether their src is a real URL or blob:/MSE, the
//                  MEASURED result of a cross-origin window.top read, and
//                  30 s MutationObserver cost counters. Stored under
//                  sender.frameId — the join key back to detections.
//
// To dump: click the probe's toolbar icon while the tab under test is
// active. The dump prints in THIS worker's console (chrome://extensions →
// "Rustloader SPIKE probe" → "service worker") and leads with:
//   SITE_VERDICT <host>: {DIRECT:…, FRAME_SCOPED:…, TIMING_ONLY:…, UNMAPPABLE:…} …
//   MUTOBS frame <id> (<origin>): callbacks=… rate=…/s …
// followed by one graded line per detection, then the full JSON.
//
// ── Machine-decided mappability grades (computed at dump time) ──
// For each sniffer-positive detection, join on frameId against that
// frame's census:
//   DIRECT       — a <video> in the SAME frame has currentSrc/src equal to
//                  the detected URL, and the frame is the top frame or can
//                  read window.top (same-origin chain) → anchorable.
//   FRAME_SCOPED — the detection's frame has ≥1 <video> but its measured
//                  window.top read threw (cross-origin child, the
//                  YouTube-embed case) → anchorable only to the <iframe>
//                  box from the parent, not to the player itself. (A URL
//                  match inside an unreachable frame still grades
//                  FRAME_SCOPED: identity without reachability doesn't
//                  make it top-anchorable.)
//   TIMING_ONLY  — the frame is reachable and has ≥1 <video>, but no URL
//                  equality exists (blob:/MSE src) → association would
//                  rest on timing/frameId correlation, not identity →
//                  fragile.
//   UNMAPPABLE   — the detection's frame census shows no <video> at all
//                  (segment-only traffic, background fetch, closed shadow
//                  root), or no census ever arrived from that frame →
//                  corner pill is the only option.

import { classify } from "./media-filter.js";

const MAX_DETECTIONS = 100;
const MAX_FRAME_EVENTS = 60;

const key = (tabId) => `probe-tab-${tabId}`;

// Same serialisation trick as the real sniffer: storage.session
// read-modify-write is racy under concurrent webRequest events.
let chain = Promise.resolve();
const serialised = (fn) => (chain = chain.then(fn, fn));

async function withRecord(tabId, fn) {
  const k = key(tabId);
  const stored = await chrome.storage.session.get(k);
  const rec = stored[k] ?? {
    frameEvents: [],
    detections: [],
    frameReports: {},
  };
  fn(rec);
  await chrome.storage.session.set({ [k]: rec });
}

function contentTypeOf(details) {
  const h = (details.responseHeaders ?? []).find(
    (x) => x.name.toLowerCase() === "content-type",
  );
  return h?.value ?? "";
}

chrome.webRequest.onHeadersReceived.addListener(
  (details) => {
    if (details.tabId < 0) return;
    const t = Date.now();

    if (details.type === "main_frame") {
      // New top document: start a fresh record (same policy as the sniffer).
      serialised(() =>
        chrome.storage.session.set({
          [key(details.tabId)]: {
            startedAt: t,
            pageUrl: details.url,
            frameEvents: [],
            detections: [],
            frameReports: {},
          },
        }),
      );
      return;
    }

    const verdict = classify({
      url: details.url,
      contentType: contentTypeOf(details),
    });
    const isSubFrame = details.type === "sub_frame";
    const chromeSaysMedia = details.type === "media";
    if (!isSubFrame && !verdict.media && !chromeSaysMedia) return;

    serialised(() =>
      withRecord(details.tabId, (rec) => {
        if (isSubFrame) {
          rec.frameEvents.push({
            t,
            url: details.url,
            frameId: details.frameId,
            parentFrameId: details.parentFrameId,
            frameType: details.frameType ?? null,
            initiator: details.initiator ?? null,
          });
          if (rec.frameEvents.length > MAX_FRAME_EVENTS)
            rec.frameEvents.shift();
        }
        if (
          (verdict.media || chromeSaysMedia) &&
          !rec.detections.some((d) => d.url === details.url)
        ) {
          rec.detections.push({
            t,
            url: details.url,
            resourceType: details.type,
            frameId: details.frameId,
            parentFrameId: details.parentFrameId,
            frameType: details.frameType ?? null,
            documentId: details.documentId ?? null,
            initiator: details.initiator ?? null,
            contentType: contentTypeOf(details),
            filterVerdict: verdict.media ? verdict.kind : `no (${verdict.reason})`,
          });
          if (rec.detections.length > MAX_DETECTIONS) rec.detections.shift();
        }
      }),
    );
  },
  { urls: ["http://*/*", "https://*/*"] },
  ["responseHeaders"],
);

// DOM census reports from probe-frame.js. sender.frameId is the join key
// between "what the DOM shows" and "where the network detection happened".
chrome.runtime.onMessage.addListener((message, sender) => {
  if (message?.type !== "frame-report" || sender.tab?.id == null) return;
  serialised(() =>
    withRecord(sender.tab.id, (rec) => {
      rec.frameReports[String(sender.frameId ?? -1)] = {
        ...message.report,
        senderOrigin: sender.origin ?? null,
        senderUrl: sender.url ?? null,
      };
    }),
  );
});

chrome.tabs.onRemoved.addListener((tabId) => {
  serialised(() => chrome.storage.session.remove(key(tabId)));
});

// ── The auto-join grader ──

const isReachable = (report) =>
  report.isTop === true || report.topAccess?.ok === true;

const framesWithVideos = (frameReports) =>
  Object.entries(frameReports)
    .filter(([, r]) => (r.videoCount ?? 0) > 0)
    .map(([id]) => id)
    .join(", ");

function gradeDetection(d, frameReports) {
  const r = frameReports[String(d.frameId)];
  if (!r) {
    return {
      grade: "UNMAPPABLE",
      reason:
        `no DOM census from frameId ${d.frameId} — content script never ` +
        `ran or frame is gone (frames with <video>: [` +
        `${framesWithVideos(frameReports)}])`,
    };
  }
  const vids = r.videos ?? [];
  const urlMatch = vids.find(
    (v) => v.currentSrc === d.url || v.srcResolved === d.url,
  );
  const reachable = isReachable(r);
  if (urlMatch && reachable) {
    return {
      grade: "DIRECT",
      reason: `URL equals <video> src in ${
        r.isTop ? "the top frame" : `same-origin-reachable frame ${d.frameId}`
      }`,
    };
  }
  if (vids.length > 0 && !reachable) {
    return {
      grade: "FRAME_SCOPED",
      reason:
        `frame ${d.frameId} (${r.origin}) has ${vids.length} <video> but ` +
        `top access failed: ${r.topAccess?.error ?? "no measurement"}` +
        (urlMatch ? " (URL even matches — identity without reachability)" : ""),
    };
  }
  if (vids.length > 0) {
    const srcs = vids
      .map((v) => (v.srcIsBlob ? "blob:" : (v.currentSrc ?? "empty")))
      .join(", ");
    return {
      grade: "TIMING_ONLY",
      reason:
        `frame reachable with ${vids.length} <video>, but no URL identity ` +
        `— video src: ${srcs.slice(0, 160)}`,
    };
  }
  return {
    grade: "UNMAPPABLE",
    reason:
      `frame ${d.frameId} (${r.origin}) census has no <video>` +
      ` (frames with <video>: [${framesWithVideos(frameReports)}])`,
  };
}

// Toolbar click → grade + dump everything for that tab to this console.
chrome.action.onClicked.addListener(async (tab) => {
  if (tab.id == null) return;
  const stored = await chrome.storage.session.get(key(tab.id));
  const rec = stored[key(tab.id)];
  const frames = await chrome.webNavigation
    .getAllFrames({ tabId: tab.id })
    .catch((e) => `getAllFrames failed: ${e?.message ?? e}`);

  console.log("──── RUSTLOADER SPIKE PROBE DUMP ────");
  if (!rec) {
    console.log(
      "no record for this tab — the tab predates the probe; reload the page and retry",
    );
    console.log("──── END DUMP ────");
    return;
  }

  // Grade only what the SHIPPED filter would have detected; Chrome-typed
  // "media" the filter rejected is kept in the JSON but counted separately.
  const sniffed = rec.detections.filter(
    (d) => !d.filterVerdict.startsWith("no ("),
  );
  const graded = sniffed.map((d) => ({
    ...d,
    ...gradeDetection(d, rec.frameReports),
  }));
  const counts = { DIRECT: 0, FRAME_SCOPED: 0, TIMING_ONLY: 0, UNMAPPABLE: 0 };
  for (const g of graded) counts[g.grade] += 1;

  const reportEntries = Object.entries(rec.frameReports);
  const withVideo = reportEntries.filter(
    ([, r]) => (r.videoCount ?? 0) > 0,
  ).length;
  const topReport = rec.frameReports["0"];
  let host = "?";
  try {
    host = new URL(tab.url ?? rec.pageUrl).hostname;
  } catch {
    // keep "?"
  }

  console.log(
    `SITE_VERDICT ${host}: {DIRECT:${counts.DIRECT}, ` +
      `FRAME_SCOPED:${counts.FRAME_SCOPED}, ` +
      `TIMING_ONLY:${counts.TIMING_ONLY}, ` +
      `UNMAPPABLE:${counts.UNMAPPABLE}} ` +
      `detections=${sniffed.length} ` +
      `filterRejected=${rec.detections.length - sniffed.length} ` +
      `framesWithVideo=${withVideo}/${reportEntries.length} ` +
      `topFrameVideo=${(topReport?.videoCount ?? 0) > 0 ? "yes" : "no"}`,
  );

  for (const [id, r] of reportEntries.slice(0, 8)) {
    const m = r.mutObs;
    if (!m) continue;
    const secs = Math.max(1, m.elapsedMs / 1000);
    console.log(
      `MUTOBS frame ${id} (${r.origin}): callbacks=${m.callbacks} ` +
        `records=${m.records} over ${secs.toFixed(0)}s ` +
        `rate=${(m.callbacks / secs).toFixed(1)}/s` +
        (m.done ? "" : " (still counting — re-dump after 30s)"),
    );
  }

  for (const g of graded) {
    console.log(
      `  ${g.grade}  frame=${g.frameId}  ${g.url.slice(0, 110)}\n` +
        `          reason: ${g.reason}`,
    );
  }

  console.log("full JSON (copy only if a grade is surprising):");
  console.log(
    JSON.stringify(
      {
        dumpedAt: new Date().toISOString(),
        tabUrl: tab.url,
        webNavigationFrames: frames,
        gradedDetections: graded,
        ...rec,
      },
      null,
      2,
    ),
  );
  console.log("──── END DUMP ────");
});
