// Rustloader Companion — options page logic (F-EXT-001 Phase 1).
// SPDX-License-Identifier: MIT (same license as rustloader itself).

import { autoPair, discover } from "./bridge-client.js";

const tokenInput = document.getElementById("token");
const statusBox = document.getElementById("status");

function showStatus(kind, message) {
  statusBox.className = kind;
  statusBox.textContent = message;
}

async function load() {
  const { bridge_token, last_error } = await chrome.storage.local.get([
    "bridge_token",
    "last_error",
  ]);
  if (bridge_token) tokenInput.value = bridge_token;
  if (last_error) {
    showStatus("warn", `Last error: ${last_error}`);
    await chrome.storage.local.remove("last_error");
  }
}

document.getElementById("auto-pair").addEventListener("click", async () => {
  showStatus("ok", "Looking for a Rustloader pairing window…");
  const result = await autoPair();
  if (result.ok) {
    const { bridge_token } = await chrome.storage.local.get("bridge_token");
    if (bridge_token) tokenInput.value = bridge_token;
    showStatus(
      "ok",
      `Paired with Rustloader ${result.version} on port ${result.port}. ` +
        "Right-click a page or use the toolbar popup to download.",
    );
  } else {
    showStatus("warn", result.message);
  }
});

document.getElementById("toggle-visibility").addEventListener("click", () => {
  const hidden = tokenInput.type === "password";
  tokenInput.type = hidden ? "text" : "password";
  document.getElementById("toggle-visibility").textContent = hidden
    ? "Hide"
    : "Show";
});

document.getElementById("save").addEventListener("click", async () => {
  const token = tokenInput.value.trim();
  if (!token) {
    showStatus("warn", "Paste the token from Rustloader's Settings first.");
    return;
  }
  await chrome.storage.local.set({ bridge_token: token });
  showStatus("ok", "Token saved. Use Test connection to confirm pairing.");
});

document.getElementById("test").addEventListener("click", async () => {
  const token = tokenInput.value.trim();
  if (!token) {
    showStatus("warn", "Paste the token from Rustloader's Settings first.");
    return;
  }
  await chrome.storage.local.set({ bridge_token: token });
  showStatus("ok", "Looking for Rustloader…");
  const found = await discover(token);
  if (!found) {
    showStatus(
      "warn",
      "Rustloader isn't reachable. Launch the app and switch on Settings → Browser Integration, then test again.",
    );
  } else if (!found.paired) {
    showStatus(
      "warn",
      `Found Rustloader ${found.version} on port ${found.port}, but it rejected this token. Re-copy it from Settings → Browser Integration.`,
    );
  } else {
    showStatus(
      "ok",
      `Paired with Rustloader ${found.version} on port ${found.port}. Right-click any page or link → “Download with Rustloader”.`,
    );
  }
});

load();
