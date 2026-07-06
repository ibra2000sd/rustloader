// Rustloader Companion — node-runnable tests for capture-gate.js.
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// The extension has no CI; this file keeps the pause/exclusion decision
// testable outside a browser:
//   node --test extension/chrome/capture-gate.test.js

import test from "node:test";
import assert from "node:assert/strict";
import { shouldCapture } from "./capture-gate.js";

test("default is capture on", () => {
  assert.equal(shouldCapture(), true);
  assert.equal(shouldCapture({}), true);
});

test("only the boolean true pauses", () => {
  assert.equal(shouldCapture({ paused: true }), false);
  assert.equal(shouldCapture({ paused: false }), true);
  assert.equal(shouldCapture({ paused: undefined }), true);
  assert.equal(shouldCapture({ paused: "true" }), true);
  assert.equal(shouldCapture({ paused: 1 }), true);
});

test("future per-site exclusion: origin in the list turns capture off", () => {
  const excludedOrigins = ["https://example.com"];
  assert.equal(
    shouldCapture({ pageOrigin: "https://example.com", excludedOrigins }),
    false,
  );
  assert.equal(
    shouldCapture({ pageOrigin: "https://other.test", excludedOrigins }),
    true,
  );
});

test("exclusion inputs must be well-formed to take effect", () => {
  // Empty/missing origin never matches; a non-array list is ignored.
  assert.equal(shouldCapture({ pageOrigin: "", excludedOrigins: [""] }), true);
  assert.equal(
    shouldCapture({
      pageOrigin: "https://example.com",
      excludedOrigins: "https://example.com",
    }),
    true,
  );
});

test("pause wins regardless of exclusion inputs", () => {
  assert.equal(
    shouldCapture({ paused: true, pageOrigin: "https://x.test", excludedOrigins: [] }),
    false,
  );
});
