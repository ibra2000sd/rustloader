// Rustloader Companion — in-page overlay, content script (gated prototype).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// Injected ONLY while the "in-page download button" option is on (dynamic
// registration in overlay-host.js) — never present otherwise. Deliberately
// dumb: it reads NO page content and holds NO bridge token; it renders the
// state the service worker pushes ({visible, count}) and relays clicks back
// by message. A classic (non-module) script, so everything lives in this
// IIFE. Design: docs/browser-integration-overlay-design.md §6.

(() => {
  "use strict";

  // Idempotence: dynamic re-registration or a worker restart must not
  // stack a second overlay into the same document.
  if (window.__rustloaderOverlayLoaded) return;
  window.__rustloaderOverlayLoaded = true;

  let host = null; // the single element we add to the page
  let listEl = null;
  let pillEl = null;
  let countEl = null;
  let panelEl = null;
  let statusEl = null;

  const CSS = `
    :host { all: initial; }
    * { box-sizing: border-box; font-family: system-ui, -apple-system, sans-serif; }
    .pill {
      display: flex; align-items: center; gap: 6px;
      background: #1b5e20; color: #fff;
      border: none; border-radius: 999px;
      padding: 8px 14px; font-size: 13px; font-weight: 600;
      cursor: pointer; box-shadow: 0 2px 10px rgba(0,0,0,0.35);
    }
    .pill:hover { background: #2e7d32; }
    .pill svg { width: 14px; height: 14px; fill: currentColor; }
    .panel {
      display: none;
      position: absolute; bottom: 42px; right: 0;
      width: 320px; max-height: 300px; overflow-y: auto;
      background: #fff; color: #222;
      border-radius: 10px; box-shadow: 0 4px 24px rgba(0,0,0,0.35);
      padding: 8px;
    }
    .panel.open { display: block; }
    .item { display: flex; align-items: center; gap: 6px; padding: 5px 4px; }
    .kind {
      flex: none; font-size: 10px; font-weight: 700; text-transform: uppercase;
      padding: 2px 6px; border-radius: 4px; background: #e8f5e9; color: #1b5e20;
    }
    .name {
      flex: 1; min-width: 0; font-size: 12px;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .item button {
      flex: none; font-size: 12px; padding: 3px 10px;
      border: 1px solid #1b5e20; border-radius: 6px;
      background: #fff; color: #1b5e20; cursor: pointer;
    }
    .item button:hover:enabled { background: #e8f5e9; }
    .item button:disabled { opacity: 0.6; cursor: default; }
    .status { font-size: 11px; color: #5f2120; padding: 4px; }
  `;

  /** Inline download-arrow icon — no web_accessible_resources needed. */
  /// Only act on a click the browser generated from a real user gesture.
  ///
  /// The page shares this document: without the check it can dispatch its own
  /// click events (or call `.click()`) on the overlay's controls and make
  /// Rustloader fetch URLs of its choosing, with no user involvement at all.
  /// `isTrusted` is set by the browser and cannot be forged from page script.
  function isRealClick(event) {
    return event instanceof Event && event.isTrusted === true;
  }

  function iconSvg() {
    const NS = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(NS, "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    const arrow = document.createElementNS(NS, "path");
    arrow.setAttribute(
      "d",
      "M12 3v10.6l-3.3-3.3-1.4 1.4L12 16.4l4.7-4.7-1.4-1.4-2.3 2.3V3h-2z",
    );
    const tray = document.createElementNS(NS, "path");
    tray.setAttribute("d", "M4 19h16v2H4z");
    svg.append(arrow, tray);
    return svg;
  }

  function displayName(url) {
    try {
      const u = new URL(url);
      const base = u.pathname.substring(u.pathname.lastIndexOf("/") + 1);
      return base ? `${u.hostname}/…/${base}` : u.hostname + u.pathname;
    } catch {
      return url;
    }
  }

  function ensureHost() {
    if (host) return;
    host = document.createElement("div");
    // Style the host inline (outside the shadow root, but page CSS aimed at
    // page markup shouldn't hit an unclassed div; the shadow root protects
    // everything inside).
    host.style.cssText =
      "position:fixed;bottom:16px;right:16px;z-index:2147483647;";
    // Closed: page script cannot reach in through `.shadowRoot` to find and
    // synthesise clicks on the controls. Not a boundary on its own — the page
    // can still remove the host — but it removes the easy handle.
    const root = host.attachShadow({ mode: "closed" });

    const style = document.createElement("style");
    style.textContent = CSS;

    pillEl = document.createElement("button");
    pillEl.className = "pill";
    pillEl.type = "button";
    pillEl.title = "Media detected — download with Rustloader";
    countEl = document.createElement("span");
    pillEl.append(iconSvg(), countEl);
    pillEl.addEventListener("click", (event) => {
      if (!isRealClick(event)) return;
      togglePanel();
    });

    panelEl = document.createElement("div");
    panelEl.className = "panel";
    listEl = document.createElement("div");
    statusEl = document.createElement("div");
    statusEl.className = "status";
    statusEl.hidden = true;
    panelEl.append(listEl, statusEl);

    root.append(style, panelEl, pillEl);
    (document.body ?? document.documentElement).append(host);
  }

  function render(visible, count) {
    if (!visible) {
      if (host) {
        host.remove();
        host = null;
      }
      return;
    }
    ensureHost();
    countEl.textContent = String(count);
  }

  function showStatus(message) {
    statusEl.hidden = false;
    statusEl.textContent = message;
  }

  async function togglePanel() {
    if (panelEl.classList.contains("open")) {
      panelEl.classList.remove("open");
      return;
    }
    listEl.replaceChildren();
    statusEl.hidden = true;
    let items = [];
    try {
      ({ items = [] } = await chrome.runtime.sendMessage({
        type: "overlay-get-list",
      }));
    } catch {
      // Worker unreachable (extension reloading); leave the list empty.
    }
    for (const item of [...items].reverse()) {
      const row = document.createElement("div");
      row.className = "item";
      const kind = document.createElement("span");
      kind.className = "kind";
      kind.textContent = item.kind;
      const name = document.createElement("span");
      name.className = "name";
      name.textContent = displayName(item.url);
      name.title = item.url;
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = "Download";
      button.addEventListener("click", (event) => {
        if (!isRealClick(event)) return;
        download(item.url, button);
      });
      row.append(kind, name, button);
      listEl.append(row);
    }
    panelEl.classList.add("open");
  }

  async function download(url, button) {
    button.disabled = true;
    button.textContent = "Sending…";
    let result;
    try {
      result = await chrome.runtime.sendMessage({
        type: "overlay-download",
        url,
      });
    } catch {
      result = { ok: false, message: "Extension unavailable — try again." };
    }
    if (result?.ok) {
      button.textContent = "Sent ✓";
      statusEl.hidden = true;
    } else {
      button.disabled = false;
      button.textContent = "Download";
      showStatus(result?.message ?? "Failed.");
    }
  }

  chrome.runtime.onMessage.addListener((message) => {
    if (message?.type === "overlay-state") {
      render(message.visible === true, Number(message.count) || 0);
    }
  });

  // Initial state (the page may have started playing before injection).
  chrome.runtime
    .sendMessage({ type: "overlay-get-state" })
    .then((state) => render(state?.visible === true, Number(state?.count) || 0))
    .catch(() => {});
})();
