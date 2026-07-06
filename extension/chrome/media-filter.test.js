// Rustloader Companion — node-runnable tests for media-filter.js.
// SPDX-License-Identifier: MIT (same license as rustloader itself).
//
// The extension has no CI; this file keeps the sniffer's filtering logic
// testable outside a browser:  node --test extension/chrome/

import test from "node:test";
import assert from "node:assert/strict";
import { classify, normalizeContentType } from "./media-filter.js";

test("HLS manifests match by content-type regardless of URL", () => {
  for (const ct of [
    "application/x-mpegURL",
    "application/vnd.apple.mpegurl",
    "audio/mpegurl",
    "audio/x-mpegurl; charset=utf-8",
  ]) {
    assert.deepEqual(
      classify({ url: "https://cdn.example.com/master?tok=1", contentType: ct }),
      { media: true, kind: "hls" },
      ct,
    );
  }
});

test("HLS/DASH match by extension even with generic content-type", () => {
  assert.deepEqual(
    classify({
      url: "https://cdn.example.com/v/master.m3u8?sig=abc",
      contentType: "application/octet-stream",
    }),
    { media: true, kind: "hls" },
  );
  assert.deepEqual(
    classify({
      url: "https://cdn.example.com/v/stream.mpd",
      contentType: "text/plain",
    }),
    { media: true, kind: "dash" },
  );
});

test("DASH manifest content-type", () => {
  assert.deepEqual(
    classify({ url: "https://x.test/manifest", contentType: "application/dash+xml" }),
    { media: true, kind: "dash" },
  );
});

test("direct video and audio match by content-type", () => {
  assert.deepEqual(
    classify({ url: "https://x.test/clip", contentType: "video/mp4" }),
    { media: true, kind: "video" },
  );
  assert.deepEqual(
    classify({ url: "https://x.test/track", contentType: "audio/mpeg" }),
    { media: true, kind: "audio" },
  );
});

test("mp4/webm match by extension when content-type is unhelpful", () => {
  assert.deepEqual(
    classify({
      url: "https://x.test/files/movie.mp4",
      contentType: "application/octet-stream",
    }),
    { media: true, kind: "video" },
  );
  assert.deepEqual(
    classify({ url: "https://x.test/v/clip.webm", contentType: "" }),
    { media: true, kind: "video" },
  );
});

test("segment noise is dropped: .ts, .m4s, mpeg-ts content-type", () => {
  assert.equal(classify({ url: "https://c.test/v/00042.ts", contentType: "video/mp2t" }).media, false);
  assert.equal(classify({ url: "https://c.test/v/00042.ts", contentType: "" }).media, false);
  assert.equal(
    classify({ url: "https://c.test/v/audio_00007.m4s", contentType: "video/iso.segment" }).media,
    false,
  );
  // mpeg-ts content-type on an extension-less URL is still a segment.
  assert.equal(classify({ url: "https://c.test/v/frag", contentType: "video/MP2T" }).media, false);
});

test("segment-name patterns are dropped even as .mp4", () => {
  for (const url of [
    "https://c.test/v/init.mp4",
    "https://c.test/v/video-init.mp4",
    "https://c.test/v/seg-00001.mp4",
    "https://c.test/v/segment12.mp4",
    "https://c.test/v/chunk_5.webm",
    "https://c.test/v/fragment-9.mp4",
  ]) {
    assert.equal(classify({ url, contentType: "video/mp4" }).media, false, url);
  }
});

test("ordinary pages and assets are not media", () => {
  for (const [url, ct] of [
    ["https://x.test/watch?v=1", "text/html"],
    ["https://x.test/app.js", "application/javascript"],
    ["https://x.test/style.css", "text/css"],
    ["https://x.test/pic.jpg", "image/jpeg"],
    ["https://x.test/data.json", "application/json"],
  ]) {
    assert.equal(classify({ url, contentType: ct }).media, false, url);
  }
});

test("a movie whose *name* merely contains 'segment' inside a word survives", () => {
  // The pattern anchors on separators/digits: "…-segment.mp4" is filtered,
  // but "…segmented-worm.mp4" is not.
  assert.equal(
    classify({ url: "https://x.test/v/segmented-worm.mp4", contentType: "video/mp4" }).media,
    true,
  );
});

test("normalizeContentType strips parameters and case", () => {
  assert.equal(normalizeContentType("Video/MP4; codecs=avc1"), "video/mp4");
  assert.equal(normalizeContentType(undefined), "");
});

test("malformed URLs never throw", () => {
  assert.equal(classify({ url: "not a url", contentType: "video/mp4" }).media, true);
  assert.equal(classify({ url: "not a url", contentType: "" }).media, false);
});
