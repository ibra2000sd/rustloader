// Rustloader Companion — background service worker (F-EXT-001 Phase 2).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// One context-menu item on pages, links, and media elements: gather the
// target URL plus the page's cookies and hand them to the running Rustloader
// app via the loopback bridge. Importing sniffer.js registers the
// observe-only webRequest media sniffer (per-tab detected list + badge).

import { sendDownload } from "./bridge-client.js";
import { collectCookies } from "./cookies.js";
import "./sniffer.js";

const MENU_ID = "rustloader-download";

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: MENU_ID,
    title: "Download with Rustloader",
    // video/audio cover right-clicks directly on media elements (srcUrl) —
    // the sniffed-media list itself lives in the toolbar popup.
    contexts: ["page", "link", "video", "audio"],
  });
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== MENU_ID) return;

  // A right-clicked media element downloads its source; a link, its target;
  // anywhere else, the page.
  const url = info.srcUrl || info.linkUrl || info.pageUrl || tab?.url;
  if (!url || !/^https?:\/\//i.test(url)) {
    await flashBadge("!", "#c0392b", tab?.id);
    return;
  }

  const pageUrl = info.pageUrl || tab?.url || url;
  const cookies = await collectCookies(pageUrl);

  const result = await sendDownload({
    url,
    page_url: pageUrl,
    cookies,
  });

  if (result.ok) {
    await flashBadge("✓", "#2e7d32", tab?.id);
  } else {
    await flashBadge("!", "#c0392b", tab?.id);
    // Surface the reason where the user will look next: the options page
    // shows the last error, and the console keeps it for debugging.
    console.warn("Rustloader Companion:", result.message);
    await chrome.storage.local.set({ last_error: result.message });
  }
});

/**
 * Flash ✓/! on the action badge. Per-tab when a tabId is known (a per-tab
 * value overrides the sniffer's per-tab count, so a global flash would be
 * invisible there); afterwards the sniffer count for that tab is restored.
 */
async function flashBadge(textValue, color, tabId) {
  const target = tabId != null ? { tabId } : {};
  try {
    await chrome.action.setBadgeBackgroundColor({ ...target, color });
    await chrome.action.setBadgeText({ ...target, text: textValue });
    setTimeout(async () => {
      try {
        let restored = "";
        if (tabId != null) {
          const key = `tab-${tabId}`;
          const stored = await chrome.storage.session.get(key);
          const count = (stored[key] ?? []).length;
          if (count > 0) restored = String(count);
          await chrome.action.setBadgeBackgroundColor({ tabId, color: "#2e7d32" });
        }
        await chrome.action.setBadgeText({ ...target, text: restored });
      } catch {
        // Tab may be gone by now.
      }
    }, 4000);
  } catch {
    // Badge is cosmetic; never let it break the flow.
  }
}
