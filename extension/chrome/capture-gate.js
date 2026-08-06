// Rustloader Companion — capture on/off decision (per-tab pause).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// PURE module: no chrome.* APIs, so `node --test` can exercise it
// (capture-gate.test.js).
//
// The single gate the sniffer consults before
// recording a detection. Today the only live input is the per-tab pause
// flag (set from the popup, cleared by the sniffer's main_frame reset so a
// reload re-enables capture). A future persistent per-site exclusion
// (F-EXT-001 follow-up in docs/ai-os/backlog.md) layers on here: pass the
// page's origin and the stored exclusion list — no new gate site in
// sniffer.js needed.

/**
 * @param {{paused?: unknown, pageOrigin?: string, excludedOrigins?: unknown}}
 *   state  only the boolean true pauses; an origin is excluded only when it
 *          appears verbatim in the list
 * @returns {boolean} true when the sniffer should record detections
 */
export function shouldCapture({ paused, pageOrigin, excludedOrigins } = {}) {
  if (paused === true) return false;
  if (
    typeof pageOrigin === "string" &&
    pageOrigin !== "" &&
    Array.isArray(excludedOrigins) &&
    excludedOrigins.includes(pageOrigin)
  ) {
    return false;
  }
  return true;
}
