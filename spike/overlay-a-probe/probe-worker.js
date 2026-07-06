// ─── THROWAWAY SPIKE INSTRUMENTATION — NEVER SHIP, NEVER MERGE ───
// Branch: spike/overlay-a-investigation. Purpose: gather the per-frame
// evidence docs/spike-overlay-a-findings.md needs (can a sniffed URL be
// associated with the visible <video> player?). Standalone diagnostic
// extension — the shipped Rustloader Companion (extension/chrome/) is
// untouched.
//
// What it records, per tab, in chrome.storage.session:
//   frameEvents  — every sub_frame document response: the frame tree as the
//                  network layer sees it (frameId, parentFrameId, url).
//   detections   — every response media-filter.js classifies as media, PLUS
//                  any response Chrome itself types as "media" even when the
//                  filter says no. Each entry keeps frameId / frameType /
//                  documentId / initiator / resourceType — exactly the
//                  fields today's sniffer (extension/chrome/sniffer.js)
//                  throws away.
//   frameReports — DOM census messages from probe-frame.js (all_frames):
//                  per frame, the <video> elements that frame can see and
//                  whether their src is a real URL or blob:/MSE. Stored
//                  under sender.frameId — the join key back to detections.
//
// To dump: click the probe's toolbar icon while the tab under test is
// active. The full JSON report (plus webNavigation.getAllFrames) prints in
// THIS worker's console: chrome://extensions → "Rustloader SPIKE probe" →
// the "service worker" link.

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
            frameType: details.frameType,
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
            frameType: details.frameType,
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

// Toolbar click → dump everything for that tab to the worker console.
chrome.action.onClicked.addListener(async (tab) => {
  if (tab.id == null) return;
  const stored = await chrome.storage.session.get(key(tab.id));
  const frames = await chrome.webNavigation
    .getAllFrames({ tabId: tab.id })
    .catch((e) => `getAllFrames failed: ${e?.message ?? e}`);
  const report = {
    dumpedAt: new Date().toISOString(),
    tabUrl: tab.url,
    webNavigationFrames: frames,
    ...(stored[key(tab.id)] ?? { note: "no record for this tab" }),
  };
  console.log("──── RUSTLOADER SPIKE PROBE DUMP ────");
  console.log(JSON.stringify(report, null, 2));
  console.log("──── END DUMP (copy everything between the markers) ────");
});
