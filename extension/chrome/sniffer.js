// Rustloader Companion — media sniffer (F-EXT-001 Phase 2).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// OBSERVE-ONLY: chrome.webRequest without blocking (MV3 removed blocking
// webRequest for regular extensions; observation is unaffected — verified
// against developer.chrome.com 2026-07). We listen to onHeadersReceived so
// the response Content-Type can drive classification (media-filter.js),
// keep a per-tab list of detected media in chrome.storage.session (in-memory,
// never written to disk, cleared when the browser closes — the right home
// for ephemeral per-tab state given the service worker's own lifetime), and
// mirror the count into the per-tab action badge (auto-cleared by Chrome
// when the tab closes).

import { classify } from "./media-filter.js";

/** Cap per tab so a pathological page can't grow storage without bound. */
const MAX_PER_TAB = 50;

const tabKey = (tabId) => `tab-${tabId}`;

// storage.session read-modify-write is racy under concurrent events; a
// module-level promise chain serialises them. (If the worker is suspended
// mid-chain the next event starts a fresh chain — acceptable: worst case a
// detection is dropped, never corrupted.)
let chain = Promise.resolve();
function serialised(fn) {
  chain = chain.then(fn, fn);
  return chain;
}

function contentTypeOf(details) {
  const header = (details.responseHeaders ?? []).find(
    (h) => h.name.toLowerCase() === "content-type",
  );
  return header?.value ?? "";
}

async function resetTab(tabId) {
  await chrome.storage.session.remove(tabKey(tabId));
  try {
    await chrome.action.setBadgeText({ tabId, text: "" });
  } catch {
    // The tab may already be gone; the badge is per-tab and auto-clears.
  }
}

async function recordDetection(tabId, item) {
  const key = tabKey(tabId);
  const stored = await chrome.storage.session.get(key);
  const list = stored[key] ?? [];
  if (list.some((m) => m.url === item.url)) return;
  list.push(item);
  if (list.length > MAX_PER_TAB) list.shift();
  await chrome.storage.session.set({ [key]: list });
  try {
    await chrome.action.setBadgeBackgroundColor({ tabId, color: "#2e7d32" });
    await chrome.action.setBadgeText({ tabId, text: String(list.length) });
  } catch {
    // Badge is cosmetic; never let it break detection.
  }
}

chrome.webRequest.onHeadersReceived.addListener(
  (details) => {
    // Requests not tied to a tab (extensions, workers) have tabId -1.
    if (details.tabId < 0) return;

    // A main-frame navigation starts a new page: its old detections are
    // stale. Doing this here avoids a webNavigation permission.
    if (details.type === "main_frame") {
      serialised(() => resetTab(details.tabId));
      return;
    }

    const verdict = classify({
      url: details.url,
      contentType: contentTypeOf(details),
    });
    if (!verdict.media) return;

    serialised(() =>
      recordDetection(details.tabId, {
        url: details.url,
        kind: verdict.kind,
        detectedAt: Date.now(),
      }),
    );
  },
  { urls: ["http://*/*", "https://*/*"] },
  ["responseHeaders"],
);

chrome.tabs.onRemoved.addListener((tabId) => {
  serialised(() => chrome.storage.session.remove(tabKey(tabId)));
});
