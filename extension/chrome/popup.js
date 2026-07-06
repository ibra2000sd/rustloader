// Rustloader Companion — toolbar popup logic (F-EXT-001 Phase 2).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// Shows the sniffer's per-tab detected-media list (chrome.storage.session,
// written by sniffer.js) with a quality/format choice, and sends the chosen
// item through the bridge exactly like the context menu does — page cookies
// included, loopback only.

import { sendDownload } from "./bridge-client.js";
import { collectCookies } from "./cookies.js";

const listEl = document.getElementById("list");
const emptyEl = document.getElementById("empty");
const statusEl = document.getElementById("status");
const qualityEl = document.getElementById("quality");
const formatEl = document.getElementById("format");

function showStatus(message, isError) {
  statusEl.hidden = false;
  statusEl.textContent = message;
  statusEl.className = isError ? "error" : "";
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

async function activeTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab ?? null;
}

async function download(item, tab, button) {
  button.disabled = true;
  button.textContent = "Sending…";
  const pageUrl = tab.url ?? item.url;
  const payload = {
    url: item.url,
    page_url: pageUrl,
    cookies: await collectCookies(pageUrl),
  };
  if (qualityEl.value) payload.quality = qualityEl.value;
  if (formatEl.value) payload.output_format = formatEl.value;

  const result = await sendDownload(payload);
  if (result.ok) {
    button.textContent = "Sent ✓";
    showStatus("Handed to Rustloader — check its Downloads view.", false);
  } else {
    button.disabled = false;
    button.textContent = "Download";
    showStatus(result.message, true);
  }
}

async function render() {
  const tab = await activeTab();
  if (!tab || tab.id == null) {
    emptyEl.hidden = false;
    return;
  }
  const key = `tab-${tab.id}`;
  const stored = await chrome.storage.session.get(key);
  const items = stored[key] ?? [];

  if (items.length === 0) {
    emptyEl.hidden = false;
    return;
  }

  // Newest first: the stream the user just started playing tops the list.
  for (const item of [...items].reverse()) {
    const row = document.createElement("div");
    row.className = "item";

    const kind = document.createElement("span");
    kind.className = `kind ${item.kind}`;
    kind.textContent = item.kind;

    const name = document.createElement("span");
    name.className = "name";
    name.textContent = displayName(item.url);
    name.title = item.url;

    const button = document.createElement("button");
    button.textContent = "Download";
    button.addEventListener("click", () => download(item, tab, button));

    row.append(kind, name, button);
    listEl.append(row);
  }
}

render();
