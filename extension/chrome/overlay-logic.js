// Rustloader Companion — overlay visibility decision (gated prototype).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// PURE module: no chrome.* APIs, so `node --test` can exercise it.
// The contract (docs/browser-integration-overlay-design.md §6): the in-page
// overlay is visible iff the overlay_enabled option is on AND the sniffer
// has at least one detection for the tab. overlay-host.js applies this on
// the worker side; the content script only renders what it is sent.

/**
 * @param {unknown} enabled  the stored overlay_enabled value (only the
 *                           boolean true counts as on)
 * @param {unknown} detections  the sniffer's per-tab detection list
 * @returns {{visible: boolean, count: number}}
 */
export function overlayState(enabled, detections) {
  const count = Array.isArray(detections) ? detections.length : 0;
  return { visible: enabled === true && count > 0, count };
}
