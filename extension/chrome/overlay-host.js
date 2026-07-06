// Rustloader Companion — in-page overlay, worker side (gated prototype).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// Design + threat model: docs/browser-integration-overlay-design.md.
// OFF by default: overlay.js is dynamically registered ONLY while the
// overlay_enabled option is on — with it off (the default) no content
// script exists in any page. The content script never receives the bridge
// token, never reads storage, and never talks to 127.0.0.1: it renders
// state pushed from here and relays clicks back; downloads go through the
// exact sendDownload path the popup and context menu use.

import { overlayState } from "./overlay-logic.js";
import { sendDownload } from "./bridge-client.js";
import { collectCookies } from "./cookies.js";

const SCRIPT_ID = "rustloader-overlay";

async function overlayEnabled() {
  const { overlay_enabled } = await chrome.storage.local.get("overlay_enabled");
  return overlay_enabled === true;
}

async function detectionsFor(tabId) {
  const key = `tab-${tabId}`;
  const stored = await chrome.storage.session.get(key);
  return stored[key] ?? [];
}

/**
 * Make the dynamic registration match the setting. Runs at every worker
 * start (registrations persist across sessions; the setting is the truth)
 * and on every overlay_enabled change.
 */
async function syncRegistration() {
  const enabled = await overlayEnabled();
  const registered = await chrome.scripting.getRegisteredContentScripts({
    ids: [SCRIPT_ID],
  });
  if (enabled && registered.length === 0) {
    await chrome.scripting.registerContentScripts([
      {
        id: SCRIPT_ID,
        js: ["overlay.js"],
        // http(s) only — mirrors the sniffer's listener filter.
        matches: ["http://*/*", "https://*/*"],
        runAt: "document_idle",
        // Defaults kept deliberately: ISOLATED world, top frame only.
      },
    ]);
  } else if (!enabled && registered.length > 0) {
    await chrome.scripting.unregisterContentScripts({ ids: [SCRIPT_ID] });
  }
}

async function pushState(tabId, detections) {
  const state = overlayState(await overlayEnabled(), detections);
  try {
    await chrome.tabs.sendMessage(tabId, { type: "overlay-state", ...state });
  } catch {
    // No overlay in that tab (page predates enabling, chrome:// page, …).
  }
}

// Unregistering does not remove already-injected scripts (per the
// chrome.scripting docs), so on disable tell live overlays to hide now.
async function hideEverywhere() {
  const tabs = await chrome.tabs.query({});
  await Promise.all(
    tabs.map((tab) =>
      tab.id == null
        ? null
        : chrome.tabs
            .sendMessage(tab.id, {
              type: "overlay-state",
              visible: false,
              count: 0,
            })
            .catch(() => {}),
    ),
  );
}

chrome.storage.local.onChanged.addListener((changes) => {
  if (!("overlay_enabled" in changes)) return;
  syncRegistration().catch((e) =>
    console.warn("Rustloader Companion: overlay registration failed:", e),
  );
  if (changes.overlay_enabled.newValue !== true) {
    hideEverywhere().catch(() => {});
  }
});

// The sniffer writes per-tab detections to storage.session; observing that
// is the overlay's only feed (sniffer.js itself stays untouched).
chrome.storage.session.onChanged.addListener((changes) => {
  for (const [key, change] of Object.entries(changes)) {
    const match = /^tab-(\d+)$/.exec(key);
    if (!match) continue;
    pushState(Number(match[1]), change.newValue ?? []).catch(() => {});
  }
});

// Requests from the content script. Only tab-attached senders are served
// (sender.tab is set for content scripts; pages themselves cannot message
// the extension — no externally_connectable entry exists).
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  const tabId = sender.tab?.id;
  if (tabId == null || typeof message?.type !== "string") return false;

  if (message.type === "overlay-get-state") {
    (async () => {
      sendResponse(overlayState(await overlayEnabled(), await detectionsFor(tabId)));
    })();
    return true; // async sendResponse
  }

  if (message.type === "overlay-get-list") {
    (async () => {
      const items = (await overlayEnabled()) ? await detectionsFor(tabId) : [];
      sendResponse({ items: items.map(({ url, kind }) => ({ url, kind })) });
    })();
    return true;
  }

  if (message.type === "overlay-download") {
    (async () => {
      // Only URLs the sniffer already recorded for THIS tab are accepted:
      // a page (or spoofed overlay) can't use the worker to send arbitrary
      // URLs to the bridge.
      const url = typeof message.url === "string" ? message.url : "";
      const known =
        (await overlayEnabled()) &&
        (await detectionsFor(tabId)).some((m) => m.url === url);
      if (!known) {
        sendResponse({ ok: false, message: "Not a detected item on this tab." });
        return;
      }
      const pageUrl = sender.tab?.url ?? url;
      sendResponse(
        await sendDownload({
          url,
          page_url: pageUrl,
          cookies: await collectCookies(pageUrl),
        }),
      );
    })();
    return true;
  }

  return false;
});

// Worker (re)start: reconcile registration with the setting.
syncRegistration().catch((e) =>
  console.warn("Rustloader Companion: overlay registration failed:", e),
);
