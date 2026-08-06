// Rustloader Companion — bridge client (F-EXT-001 Phase 1).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// Talks to the loopback bridge inside the running Rustloader app
// (src/bridge/mod.rs). Protocol: docs/browser-integration-design.md §5.

// Keep in sync with BRIDGE_PORTS in src/bridge/mod.rs.
export const BRIDGE_PORTS = [46150, 46151, 46152, 46153, 46154];

const PING_TIMEOUT_MS = 1500;

/** Fetch with a hard timeout (the bridge is local; anything slow is absent). */
async function fetchWithTimeout(url, options = {}, timeoutMs = PING_TIMEOUT_MS) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...options, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Ask one port who it is, WITHOUT sending the token. Resolves to
 * `{port, version}` when something claiming to be a Rustloader bridge answers,
 * or `null` for anything else (other app, no listener).
 *
 * Discovery probes ports the app may never have used, and anything local can
 * hold one of them, so the token must not ride along on a probe — a stranger
 * on 46150 would collect it (and then every page cookie we later send).
 * `/api/v1/ping` deliberately answers unauthenticated for exactly this.
 */
export async function identify(port) {
  try {
    const resp = await fetchWithTimeout(`http://127.0.0.1:${port}/api/v1/ping`);
    if (!resp.ok) return null;
    const body = await resp.json();
    if (body.app !== "rustloader") return null;
    return { port, version: body.version };
  } catch {
    return null;
  }
}

/**
 * Whether `token` is the one the bridge on `port` expects. Only ever call this
 * for a port `identify` has already vouched for, so the token goes to a single
 * endpoint that speaks the protocol rather than to the whole scan range.
 */
export async function isPaired(port, token) {
  if (!token) return false;
  try {
    const resp = await fetchWithTimeout(`http://127.0.0.1:${port}/api/v1/ping`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!resp.ok) return false;
    const body = await resp.json();
    return body.app === "rustloader" && body.paired === true;
  } catch {
    return false;
  }
}

/**
 * Find the port Rustloader's bridge is listening on: the cached port first,
 * then the whole range in parallel. Resolves to `{port, version}` or `null`.
 *
 * Takes no token — see `identify`. Pass the result's `port` to `isPaired` when
 * the caller needs to know whether the stored token still matches.
 */
export async function discover() {
  const { bridge_port: cached } = await chrome.storage.local.get("bridge_port");
  if (cached) {
    const hit = await identify(cached);
    if (hit) return hit;
  }
  const results = await Promise.all(BRIDGE_PORTS.map((p) => identify(p)));
  const found = results.find((r) => r !== null) ?? null;
  if (found) await chrome.storage.local.set({ bridge_port: found.port });
  return found;
}

/**
 * Auto-pair via the app's single-use pairing window (GET /api/v1/pair):
 * works only while the user has clicked "Pair" in Rustloader's Settings
 * within the last ~120 s. Probes the whole port range; a port must first
 * identify itself as rustloader via /ping before we trust its /pair.
 *
 * Resolves to `{ok: true, port, version}` (token saved to storage.local) or
 * `{ok: false, reason: "not_running"|"window_closed", message}`.
 */
export async function autoPair() {
  const pings = await Promise.all(BRIDGE_PORTS.map((p) => identify(p)));
  const found = pings.find((r) => r !== null);
  if (!found) {
    return {
      ok: false,
      reason: "not_running",
      message:
        "Rustloader isn't reachable. Launch it and switch on Settings → Browser Integration.",
    };
  }
  try {
    const resp = await fetchWithTimeout(
      `http://127.0.0.1:${found.port}/api/v1/pair`,
    );
    if (resp.ok) {
      const body = await resp.json();
      if (typeof body.token === "string" && body.token.length > 0) {
        await chrome.storage.local.set({
          bridge_token: body.token,
          bridge_port: found.port,
        });
        return { ok: true, port: found.port, version: found.version };
      }
    }
  } catch {
    // Fall through to the window-closed message.
  }
  return {
    ok: false,
    reason: "window_closed",
    message:
      "Rustloader is running, but no pairing window is open. In Rustloader: Settings → Browser Integration → Pair, then click this within 120 seconds. (Older app versions need a manual token paste.)",
  };
}

/**
 * POST a download request. `payload` is the /api/v1/download body
 * (url, cookies, quality, output_format — see the design doc).
 *
 * Resolves to `{ok: true}` or `{ok: false, reason}` where reason is one of
 * "not_running" | "unpaired" | "rejected" — plus a human `message`.
 */
export async function sendDownload(payload) {
  const { bridge_token: token } = await chrome.storage.local.get("bridge_token");
  if (!token) {
    return {
      ok: false,
      reason: "unpaired",
      message: "Not paired: open the extension options and paste the token from Rustloader's Settings.",
    };
  }
  const found = await discover();
  if (!found) {
    return {
      ok: false,
      reason: "not_running",
      message: "Rustloader isn't reachable. Launch it and switch on Settings → Browser Integration.",
    };
  }
  try {
    const resp = await fetchWithTimeout(
      `http://127.0.0.1:${found.port}/api/v1/download`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(payload),
      },
      8000,
    );
    if (resp.status === 202) return { ok: true };
    if (resp.status === 401 || resp.status === 403) {
      return {
        ok: false,
        reason: "unpaired",
        message: "Rustloader rejected the token — re-copy it from Settings → Browser Integration.",
      };
    }
    let detail = `HTTP ${resp.status}`;
    try {
      detail = (await resp.json()).error ?? detail;
    } catch {
      // non-JSON error body; keep the status text
    }
    return { ok: false, reason: "rejected", message: `Rustloader refused the request: ${detail}` };
  } catch {
    return {
      ok: false,
      reason: "not_running",
      message: "Rustloader stopped responding while sending the request.",
    };
  }
}
