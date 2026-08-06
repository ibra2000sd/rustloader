// Rustloader Companion — node-runnable tests for bridge-client.js.
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// The load-bearing property here is that the pairing token never rides on a
// discovery probe: run with  node --test extension/chrome/

import test from "node:test";
import assert from "node:assert/strict";

/** Every fetch the client made, as {port, authorization, method}. */
let calls = [];

/**
 * Install stubs for the two browser globals bridge-client.js touches.
 * `rustloaderPort` is the only port that answers as the app; every other port
 * behaves like a stranger that also claims to be rustloader, which is exactly
 * the case the token must not be exposed to.
 */
function installStubs({ rustloaderPort, storage = {} } = {}) {
  calls = [];
  const store = { ...storage };

  globalThis.chrome = {
    storage: {
      local: {
        async get(key) {
          const keys = Array.isArray(key) ? key : [key];
          return Object.fromEntries(
            keys.filter((k) => k in store).map((k) => [k, store[k]]),
          );
        },
        async set(items) {
          Object.assign(store, items);
        },
      },
    },
  };

  globalThis.fetch = async (url, options = {}) => {
    const port = Number(new URL(url).port);
    const authorization = options.headers?.Authorization ?? null;
    calls.push({ port, authorization, method: options.method ?? "GET", url });

    if (url.includes("/api/v1/ping")) {
      // Any listener can claim the name — that is the whole point.
      return jsonResponse(200, {
        app: "rustloader",
        api: 1,
        version: "0.11.0",
        paired: authorization === "Bearer good-token",
      });
    }
    if (url.includes("/api/v1/download")) {
      if (port !== rustloaderPort) return jsonResponse(401, { error: "nope" });
      return jsonResponse(202, {});
    }
    return jsonResponse(404, { error: "not found" });
  };

  return store;
}

function jsonResponse(status, body) {
  return {
    ok: status >= 200 && status < 300,
    status,
    async json() {
      return body;
    },
  };
}

const load = async () => await import("./bridge-client.js");

test("discovery never sends the token to any port", async () => {
  // A stored token is what makes this test meaningful: the pre-fix client
  // read it here and attached it to every probe.
  installStubs({
    rustloaderPort: 46150,
    storage: { bridge_token: "good-token" },
  });
  const { discover } = await load();

  const found = await discover();

  assert.equal(found.port, 46150);
  assert.ok(calls.length > 0, "discovery must actually probe");
  assert.deepEqual(
    calls.filter((c) => c.authorization !== null),
    [],
    "a discovery probe must never carry an Authorization header",
  );
});

test("a stale cached port does not leak the token while re-scanning", async () => {
  installStubs({
    rustloaderPort: 46152,
    storage: { bridge_port: 46151, bridge_token: "good-token" },
  });
  const { discover } = await load();

  await discover();

  assert.deepEqual(
    calls.filter((c) => c.authorization !== null),
    [],
    "re-scanning after a stale cache must stay unauthenticated",
  );
});

test("isPaired authenticates against exactly one already-identified port", async () => {
  installStubs({ rustloaderPort: 46150 });
  const { isPaired } = await load();

  assert.equal(await isPaired(46150, "good-token"), true);
  assert.equal(await isPaired(46150, "wrong-token"), false);

  const ports = new Set(calls.map((c) => c.port));
  assert.deepEqual([...ports], [46150], "must not fan out to the whole range");
  assert.ok(
    calls.every((c) => c.authorization !== null),
    "isPaired is the authenticated probe",
  );
});

test("isPaired without a token makes no request at all", async () => {
  installStubs({ rustloaderPort: 46150 });
  const { isPaired } = await load();

  assert.equal(await isPaired(46150, ""), false);
  assert.deepEqual(calls, []);
});

test("sendDownload only reveals the token on the download POST", async () => {
  installStubs({
    rustloaderPort: 46150,
    storage: { bridge_token: "good-token" },
  });
  const { sendDownload } = await load();

  const result = await sendDownload({ url: "https://example.com/v.mp4" });

  assert.deepEqual(result, { ok: true });
  const authenticated = calls.filter((c) => c.authorization !== null);
  assert.equal(authenticated.length, 1, "exactly one request may carry it");
  assert.equal(authenticated[0].method, "POST");
  assert.equal(authenticated[0].port, 46150);
});
