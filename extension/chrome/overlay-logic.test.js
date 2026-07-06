// Rustloader Companion — overlay-logic tests (gated prototype).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// Run:  node --test extension/chrome/overlay-logic.test.js

import test from "node:test";
import assert from "node:assert/strict";
import { overlayState } from "./overlay-logic.js";

const item = (url) => ({ url, kind: "video", detectedAt: 0 });

test("visible when enabled and detections exist", () => {
  assert.deepEqual(overlayState(true, [item("https://a/v.mp4")]), {
    visible: true,
    count: 1,
  });
});

test("count reflects the whole list", () => {
  const list = [item("https://a/1"), item("https://a/2"), item("https://a/3")];
  assert.deepEqual(overlayState(true, list), { visible: true, count: 3 });
});

test("hidden when disabled, even with detections", () => {
  assert.deepEqual(overlayState(false, [item("https://a/v.mp4")]), {
    visible: false,
    count: 1,
  });
});

test("hidden when enabled but nothing detected", () => {
  assert.deepEqual(overlayState(true, []), { visible: false, count: 0 });
});

test("hidden when the setting was never written (undefined)", () => {
  assert.deepEqual(overlayState(undefined, [item("https://a/v.mp4")]), {
    visible: false,
    count: 1,
  });
});

test("only boolean true enables — truthy junk stays off", () => {
  for (const junk of ["true", 1, {}, []]) {
    assert.equal(overlayState(junk, [item("https://a/v.mp4")]).visible, false);
  }
});

test("non-array detections (missing storage key) mean zero", () => {
  assert.deepEqual(overlayState(true, undefined), { visible: false, count: 0 });
  assert.deepEqual(overlayState(true, null), { visible: false, count: 0 });
});
