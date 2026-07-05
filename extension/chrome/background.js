// Rustloader Companion — background service worker (F-EXT-001 Phase 1).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// One context-menu item on pages and links: gather the target URL plus the
// page's cookies and hand them to the running Rustloader app via the
// loopback bridge. No media sniffing in Phase 1 (that's Phase 2).

import { sendDownload } from "./bridge-client.js";

const MENU_ID = "rustloader-download";

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: MENU_ID,
    title: "Download with Rustloader",
    contexts: ["page", "link"],
  });
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== MENU_ID) return;

  // A right-clicked link downloads the link target; anywhere else, the page.
  const url = info.linkUrl || info.pageUrl || tab?.url;
  if (!url || !/^https?:\/\//i.test(url)) {
    await flashBadge("!", "#c0392b");
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
    await flashBadge("✓", "#2e7d32");
  } else {
    await flashBadge("!", "#c0392b");
    // Surface the reason where the user will look next: the options page
    // shows the last error, and the console keeps it for debugging.
    console.warn("Rustloader Companion:", result.message);
    await chrome.storage.local.set({ last_error: result.message });
  }
});

/**
 * The page's cookies (what the design doc calls "page cookies"), converted
 * from chrome.cookies records to the bridge's cookie JSON. They only ever
 * travel to 127.0.0.1.
 */
async function collectCookies(pageUrl) {
  try {
    const cookies = await chrome.cookies.getAll({ url: pageUrl });
    return cookies.map((c) => ({
      name: c.name,
      value: c.value,
      domain: c.domain,
      path: c.path,
      secure: c.secure,
      httpOnly: c.httpOnly,
      // Absent for session cookies; the bridge writes 0 then.
      expires: c.expirationDate,
    }));
  } catch (e) {
    console.warn("Rustloader Companion: could not read cookies:", e);
    return [];
  }
}

async function flashBadge(textValue, color) {
  try {
    await chrome.action.setBadgeBackgroundColor({ color });
    await chrome.action.setBadgeText({ text: textValue });
    setTimeout(() => chrome.action.setBadgeText({ text: "" }), 4000);
  } catch {
    // Badge is cosmetic; never let it break the flow.
  }
}
