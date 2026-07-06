// Rustloader Companion — page-cookie collection (F-EXT-001).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// Shared by the background service worker (context menu) and the popup:
// the page's cookies, converted from chrome.cookies records to the bridge's
// cookie JSON (design doc §5). They only ever travel to 127.0.0.1.

export async function collectCookies(pageUrl) {
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
