// Rustloader Companion — media classification (F-EXT-001 Phase 2).
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// PURE module: no chrome.* APIs, so `node --test` can exercise it
// (media-filter.test.js). sniffer.js feeds it every response the browser
// sees; it answers "is this a downloadable media item, and what kind?"
//
// Design (docs/browser-integration-design.md §4): match media content-types
// (video/*, audio/*, HLS, DASH) and extensions (.m3u8, .mpd, .mp4, .webm);
// drop segment noise (.ts, .m4s, init/seg/chunk patterns) so the per-tab
// list holds streams, not their thousands of fragments.

/** Manifest content-types (lowercase, parameters stripped) → kind. */
const MANIFEST_TYPES = new Map([
  ["application/x-mpegurl", "hls"],
  ["application/vnd.apple.mpegurl", "hls"],
  ["audio/mpegurl", "hls"],
  ["audio/x-mpegurl", "hls"],
  ["application/dash+xml", "dash"],
]);

/** Media file extensions → kind. */
const MEDIA_EXTENSIONS = new Map([
  ["m3u8", "hls"],
  ["mpd", "dash"],
  ["mp4", "video"],
  ["webm", "video"],
]);

/** Extensions that are (nearly) always stream fragments, never the stream. */
const SEGMENT_EXTENSIONS = new Set(["ts", "m4s"]);

/** Basenames like init.mp4, seg-00042.m4s, chunk_1.webm, fragment12.mp4. */
const SEGMENT_BASENAME = /(^|[-_.])(init|seg|segment|chunk|frag|fragment)([-_.]?\d+)?$/;

/** `Content-Type` value → lowercase media type without parameters. */
export function normalizeContentType(contentType) {
  if (!contentType) return "";
  return contentType.split(";")[0].trim().toLowerCase();
}

/** Lowercase last path component of a URL, without the query/fragment. */
function basenameOf(url) {
  try {
    const path = new URL(url).pathname;
    const last = path.substring(path.lastIndexOf("/") + 1);
    return decodeURIComponent(last).toLowerCase();
  } catch {
    return "";
  }
}

/**
 * Classify one observed response.
 *
 * @param {{url: string, contentType?: string}} req
 * @returns {{media: true, kind: "hls"|"dash"|"video"|"audio"} |
 *           {media: false, reason: string}}
 */
export function classify({ url, contentType }) {
  const type = normalizeContentType(contentType);
  const basename = basenameOf(url);
  const dot = basename.lastIndexOf(".");
  const ext = dot >= 0 ? basename.substring(dot + 1) : "";
  const stem = dot >= 0 ? basename.substring(0, dot) : basename;

  // Manifests win outright: an HLS/DASH playlist is the stream itself, no
  // matter what its URL looks like.
  const manifestKind = MANIFEST_TYPES.get(type);
  if (manifestKind) return { media: true, kind: manifestKind };
  if (MEDIA_EXTENSIONS.get(ext) === "hls") return { media: true, kind: "hls" };
  if (MEDIA_EXTENSIONS.get(ext) === "dash") return { media: true, kind: "dash" };

  // Segment noise: fragments of a stream the manifest already represents.
  if (SEGMENT_EXTENSIONS.has(ext)) return { media: false, reason: "segment extension" };
  if (type === "video/mp2t") return { media: false, reason: "mpeg-ts segment" };
  if (SEGMENT_BASENAME.test(stem)) return { media: false, reason: "segment name pattern" };

  // Direct media by content-type.
  if (type.startsWith("video/")) return { media: true, kind: "video" };
  if (type.startsWith("audio/")) return { media: true, kind: "audio" };

  // Direct media by extension (e.g. octet-stream .mp4).
  const extKind = MEDIA_EXTENSIONS.get(ext);
  if (extKind) return { media: true, kind: extKind };

  return { media: false, reason: "not media" };
}
