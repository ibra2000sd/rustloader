# Backlog

IDs: `B-<AREA>-NNN` bug, `F-<AREA>-NNN` feature. Status: `open` / `in-progress` /
`closed`. Close items in the same PR that does the work, recording the merge SHA.

Most of the open items below were surfaced by the **aria2 adoption spike (audit,
2026-06-30)** — a read-only investigation whose findings are the source for the
download-reliability work.

## P1 — do first

### B-DL-001 — Segment resume must require HTTP 206, else restart · closed · SMALL-MEDIUM
Follow-up to F-DL-002 / PR #28: the resume branch in `download_segment_attempt`
(`segment.rs`) checked only `response.status().is_success()` before appending
to the existing `.partN` file, which accepts a `200 OK` as well as `206`. A
server/CDN/proxy that ignores the `Range` header and returns `200` with the
full body (cache miss, range-coalescing proxy, etc.) would get its body
appended onto the already-written bytes, silently producing an oversized,
corrupt part file with no error raised. **Fix:** on resume
(`existing_bytes > 0`), require `reqwest::StatusCode::PARTIAL_CONTENT`
(optionally cross-checked against the `Content-Range` start offset when
present) before appending; any other status truncates the stale partial and
returns `Err` so the retry loop restarts the segment fresh. The first-attempt
(`existing_bytes == 0`) path is unchanged. Regression test added:
`test_resume_restarts_when_server_ignores_range`. PR
[#29](https://github.com/ibra2000sd/rustloader/pull/29), merged `c976872`
(2026-07-01).

### F-DL-002 — Segment-failure tolerance: don't abort the whole download · closed (retry-resume half) · MEDIUM
When any single segment errors, the engine `break`s and fails the **entire**
download (`engine.rs` result loop), and per-segment retries truncate from byte 0
(`segment.rs` `File::create`). A single dropped connection to a throttled/capped
host therefore kills a large transfer. **Fix:** tolerate/retry a failed segment
without aborting the whole download, and make segment retry resume from the
already-written bytes instead of re-downloading from 0. This is the cheaper half
of audit "Shape D" and independently fixes the throttled-host failure mode.
Source: internal audit 2026-06-30.
**Status:** the retry-resume half is done — `segment.rs` retries now resume
from already-written bytes (Range + append, cumulative progress, wall-clock-
bounded retry budget) instead of truncating. PR
[#28](https://github.com/ibra2000sd/rustloader/pull/28), merged `c1c0580`
(2026-07-01). The engine's `break` on a genuinely-unrecoverable segment is
intentionally retained/unchanged — see the PR description. The
whole-download-abort-tolerance half (letting the engine survive a segment
that never recovers) remains open, tracked separately if pursued.

### B-DL-004 — Blanket 30s TOTAL client timeout killed slow native transfers · closed · MEDIUM
`DownloadEngine::new` built the reqwest client with `.timeout(30s)` — in
reqwest a **total** request timeout covering the body read. Any native
transfer whose body took >30s failed: `download_simple` could never complete
on a slow link (proven live in the master audit: a 3 MB file over a ~70 KB/s
throttled server died at ~66%), and segmented downloads had every in-flight
segment body killed at T+30s, surviving only via #28/#29's resume-append
retry churn. **Fix:** client now uses `CONNECT_TIMEOUT` (15s, handshake
only); every native HTTP *wait* (response headers, each body-chunk read, in
both `segment.rs::download_segment_attempt` and `download_simple`) is
bounded by the new `STALL_ABORT_TIMEOUT` (`progress.rs`, aligned with
`STALL_DETECTION_SECONDS` = 30s). Slow-but-progressing transfers run to
completion; a genuinely idle connection aborts within the window (I-1's
bound-the-wait rule) and feeds the existing resume-append retry on the
segmented path. Stall watchdog stays notify-only. Regression tests:
`test_simple_download_slower_than_30s_total_completes`,
`test_simple_download_stalled_aborts_bounded_and_leaves_no_final_file`,
`test_stalled_segment_read_aborts_within_bound`. Source: master audit
2026-07-01/02, finding 1. PR
[#37](https://github.com/ibra2000sd/rustloader/pull/37).

### B-DL-005 — `download_simple` left a failed partial under the FINAL name · closed · SMALL
`download_simple` `File::create`d the final output path directly, so a
mid-transfer failure left a partial (e.g. 2.1 MB of a 3.1 MB `slow.mp4`)
that looks like a completed download and plays corrupt. **Fix:** stream into
a temp part next to the output (`<file_name>.part0` — deliberately
`calculate_segments`' naming, so the queue's cancel/remove artifact cleanup
(B-DL-002/#36) and the resume-guard's stale-part discard already cover it)
and rename into place only on success; on failure remove the temp and leave
nothing under the final name. Regression tests:
`test_failed_simple_download_leaves_no_final_named_file`,
`test_simple_download_success_renames_temp_to_final`. Source: master audit
2026-07-01/02, finding 2. PR
[#37](https://github.com/ibra2000sd/rustloader/pull/37).

### B-DL-006 — Saved extension came from the mode flag, not the actual content · closed · MEDIUM
`cli.rs::output_path` picked the extension from the mode flag (`.mp3` if
`-f mp3`, else `.mp4`) **before** the download, and the GUI hardcoded `.mp4`
for every task — so a real MP3 (`audio/mpeg`) saved as `t-rex-roar.mp4` and
a Windows installer (via the yt-dlp fallback) saved as `7z2408.mp4` (audit's
live repros). The old test `output_path_uses_extension_for_format` locked
the bug in. **Fix:** the caller's extension is now provisional; the engine
finalizes it from the actual content and `DownloadEngine::download` returns
the real saved path. Native path: probe `Content-Type` → mapped extension
(`audio/mpeg`→`mp3`, `audio/mp4`→`m4a`, `video/webm`→`webm`, …), falling
back to the redirect-resolved URL path's extension for
`application/octet-stream`, else the caller's extension stands. yt-dlp path:
the literal `-o <title>.mp4` became `-o <title>.%(ext)s` so yt-dlp names the
file by its real container, and the file it actually wrote is adopted.
In-flight artifacts (`.partN`, resume sidecar) stay keyed to the provisional
name — #36 cleanup and the F-DL-003 identity guard are untouched; only the
final rename (the #37 temp→rename on the simple path, a post-merge rename on
the segmented path, best-effort) adopts the corrected name. I-8's non-media
guard is unchanged. Regression tests:
`test_simple_download_audio_mpeg_saves_as_mp3`,
`test_simple_download_octet_stream_uses_url_extension`,
`test_segmented_download_adopts_url_extension_for_octet_stream`,
`test_content_derived_output_path_priority`,
`test_ytdlp_output_template_swaps_extension`, and the upgraded
`output_path_extension_is_provisional_only`. Source: master audit
2026-07-01/02, finding 3.

### B-GUI-001 — GUI YouTube downloads dead: unimplemented native stub broke `get_direct_url` · closed (PR open) · SMALL
Pasting a YouTube URL in the GUI showed "Extracting…" then nothing — no
downloads-panel row, no queue task, no history entry, reproducibly — while the
same URL downloaded fine via CLI. Root cause (live-diagnosed 2026-07-03 on
`babe8e0`): the GUI actor registered the **unimplemented**
`NativeYoutubeExtractor` stub in its hybrid registry (`actor.rs`), and while
`HybridExtractor::extract_info` falls back to yt-dlp when the stub errors,
`get_direct_url` had **no fallback** (`hybrid.rs`) — so extraction succeeded,
the GUI sent `StartDownload`, and `get_download_url` always died on the stub's
`Err("Native extraction not implemented yet")`. The failure was fully silent:
both error exits in `handle_start_download` sent `BackendEvent::Error`
(status-bar text only) with no log line, the #47 warn only covers the
extraction leg, and the stub's `get_direct_url` (unlike its `extract_info`)
logs nothing. Not cookies (settings were empty; cookie config is opt-in,
default `None`) and not yt-dlp (the identical command succeeded in 6.5s).
**Fix:** the GUI now builds its `HybridExtractor` with an **empty native
registry**, exactly like the proven CLI path (`cli.rs`) — re-register the
native extractor only once it actually extracts; `HybridExtractor::
get_direct_url` gained the same fallback-on-error retry `extract_info` has
(defense in depth for any future native extractor); and both
`handle_start_download` error exits now `warn!` with url + error (#47 style),
closing the observability gap. Verified end-to-end at the actor level under an
isolated HOME: `ExtractInfo` → `ExtractionCompleted(Ok)` → `StartDownload` →
queue task created → segmented native download of the resolved direct URL →
31 MB file landed + history row persisted. Source: interactive GUI test
session, 2026-07-03.

### B-GUI-002 — Non-Latin titles render as tofu (□): Basic shaping blocks font fallback · closed (PR open) · SMALL
Regression from F-GUI-002/#45: bundling Geist + Geist Mono (both Latin-only)
and making Geist the `default_font` turned every Arabic/CJK/emoji video title
into □ boxes. Root cause verified against the vendored sources: iced 0.12.3
defaults every `Text` widget to `Shaping::Basic`
(`iced_core-0.12.3/src/widget/text.rs:51`), and cosmic-text 0.10.0's Basic
path (`shape_skip`, `shape.rs`) maps all chars through the **first** matched
font's charmap only — missing chars become glyph 0 (tofu), with no fallback
and no Arabic joining/RTL. The system fonts ARE in the font database
(`FontSystem::new_with_fonts` → `db.load_system_fonts()`), so
`Shaping::Advanced` (`shape_run`) already resolves missing glyphs per-script
via platform lists (macOS Geeza Pro/PingFang/Hiragino/Apple Color Emoji,
equivalents on Windows/Linux). **Fix:** `theme::SHAPING_CONTENT`
(= `Shaping::Advanced`) applied to every content-derived text site — titles
(`download_item.rs`, `history_item.rs`), error messages (incl. the ⚠/✕/💡
prefixes Geist also lacks), output path, clipboard-detected URL, history load
error. Fonts unchanged: Geist stays first in the chain, Geist Mono keeps the
numeric data. UI chrome stays on the cheaper Basic default. Known limitation:
`TextInput` in iced 0.12 exposes no shaping option, so non-ASCII typed into
the URL/path inputs can still tofu (revisit on the iced upgrade). Final
acceptance is the maintainer's visual check of an Arabic/CJK title.

### B-DL-008 — yt-dlp fallback retried the dead direct URL instead of the page URL · closed (PR open) · SMALL
Found live during the #48 GUI acceptance run (2026-07-03, `6ba62e8`): a TikTok
download failed with "yt-dlp download failed" even though yt-dlp supports
TikTok. The queue path hands the engine the extractor-**resolved direct URL**
(`actor.rs` stores it in `updated_format.url`; `manager.rs` passes
`task.format.url` to `engine.download()`), and sites with signed/session-bound
media URLs (TikTok; YouTube DASH behaves the same) 403 a fresh client — the
probe 403'd, the engine fell back to `download_via_ytdlp`, and yt-dlp's
`[generic]` extractor got the **same dead direct URL** and 403'd identically.
The `engine.rs` comment "yt-dlp runs on the page URL itself" was only true for
the CLI path, whose `download()` input IS the page URL. Control cases from the
same session: YouTube 360p progressive and a Facebook video probed fine and
downloaded natively. **Fix:** new `PageFallback { page_url, format_spec }`
carries the original page URL + a yt-dlp `-f` spec reproducing the chosen
format (`{id}+bestaudio/{id}/best` for DASH-split video-only, `{id}/best`
otherwise); `engine.download_with_fallback()` routes both yt-dlp exits (probe
failure, non-media Content-Type) through it, `download()` delegates with
`None` so the CLI path is byte-identical; the queue path builds the fallback
from `task.video_info.url` + `task.format`. `YtDlpOptions.format_spec` is the
verbatim `-f` override (outranks `quality`; `audio_only` still wins). Unit
tests cover the selector precedence, both `PageFallback` spec shapes, and the
fallback-target routing. Source: interactive GUI acceptance session,
2026-07-03.

### B-GUI-003 — Quality selector ignored: every GUI download got max resolution · closed (PR open) · SMALL-MEDIUM
Reported by the maintainer, verified at `0d53329`: the dropdown stored the
choice correctly (`app.rs` `QualityChanged` → `VideoQuality::Specific("480")`)
but it never travelled — the auto-start path sent `StartDownload` with
`format_id: None` and no quality, and `select_format(…, None)` picked max
resolution unconditionally (`max_by_key(width*height)`). Three sibling leaks:
the dropdown *displayed* `Specific(_)` as "Custom" (matches no pick_list
option, renders blank); save flattened `Specific(_)` to `"Custom"`; load
mapped anything but Best/Worst to Best — so the choice also silently reverted
across restarts. **Fix:** `StartDownload` carries `quality: VideoQuality`;
`select_format` honours it — `Best` byte-identical to the old behaviour
(progressive-first, DASH/direct fallback), `Specific(h)` picks the best
format with `height <= h` across all formats (progressive preferred at equal
resolution; nothing under the cap → nearest above; heightless direct files
still selectable), `Worst` picks the smallest with a video track. The choice
travels as a concrete format id, so the native engine downloads its resolved
URL and the B-DL-008 `PageFallback` reproduces it on the yt-dlp path —
no engine changes. Display renders `Specific(h)` as "{h}p"; persistence
round-trips the height (legacy `"Custom"` rows still load as Best).
Acceptance (real runs, isolated HOME, same YouTube video):
480→854×480, 720→1280×720, 1080→1920×1080 by ffprobe; TikTok/Facebook
downloads still succeed. Unit tests cover the cap, tie-preference, nearest-
above, heightless, Worst-not-audio, id-outranks-quality, and the settings
round-trip; `tests/quality_acceptance.rs` (`#[ignore]`, real network) encodes
the acceptance. Known limitation, pre-existing and unchanged: `Best` still
means "best progressive", so YouTube `Best` yields 360p (its only progressive
format) while `Specific(1080)` yields 1080p — filed as an adjacent
observation, not fixed here. Source: maintainer bug report + fix session,
2026-07-03.

### B-GUI-004 — "Best Available" gave 360p on YouTube; video-only picks downloaded silent · closed (PR open) · SMALL
The B-GUI-003 known limitation, promoted to a fix: `select_format`'s `Best`
arm preferred the best *progressive* (video+audio) format, and YouTube serves
no progressive above 360p (everything higher is DASH-split) — so the DEFAULT
quality delivered 360p while `Specific(1080)` delivered real 1080p. **Second
finding, live at `f14acaa` during this fix's acceptance:** the DASH merge
believed to make video-only picks whole (`PageFallback`'s
`{id}+bestaudio/{id}/best`) only runs when the native probe FAILS — TikTok's
session-bound URLs do fail it, but YouTube's IP-bound direct URLs probe fine
from the extracting machine, so the native engine downloaded the lone video
stream: at `f14acaa`, `Specific(1080)` produced a SILENT 1920×1080 file
(ffprobe: av1 video, no audio stream). B-GUI-003's acceptance had only
compared heights. **Fix (both, `actor.rs` only):** (1) the `Best` arm now
selects the highest-resolution format across ALL formats — `Specific` with no
height cap, mirroring yt-dlp's default `bv*+ba/b`; progressive still wins
ties, heightless direct files / audio-only formats (area 0) stay pickable —
never an error where the old arm succeeded. Deliberate behaviour change; the
unit tests pinning best-progressive were updated. (2) `get_download_url`
returns the PAGE URL for a video-only pick (the same move as its HLS branch),
so the engine's probe sees HTML and routes to yt-dlp, where the existing
`PageFallback` spec downloads video+audio and ffmpeg merges them — no engine
or queue changes. Acceptance (real isolated runs, ffprobe on each): YouTube
Best → 3840×1620 (the video's true max) WITH opus audio, yt-dlp `-f
401+bestaudio/401/best` in the log; 480/720/1080 → exact heights, now WITH
audio; TikTok Best → 1080×1920 + aac via its combined format (no forced
split); Facebook Best → 720p + aac on the native path; a direct `.mp3` and a
direct `.mp4` still take the native path (probed `audio/mpeg` / `video/mp4`).
ffmpeg (a documented prerequisite) performs the merges. Source: maintainer
report + fix session, 2026-07-03.

### B-DL-009 — .app + cookies: no JS runtime → zero formats, flattened to the generic error · closed (PR open) · SMALL
Diagnosed read-only 2026-07-04 on the installed 0.9.0 `.app`: with the GUI's
persisted `cookies_from_browser=chrome`, every YouTube extraction failed as
"Unable to process this URL. Please try a different video", while terminal
runs worked. Root cause is a three-factor combination, NOT a missing bundled
yt-dlp (that was found, unquarantined, and executable — a full no-PATH
`env -i` download succeeded): (1) the GUI applies the persisted cookie source
to every extraction (`actor.rs`), unlike the CLI (flags only); (2) with
cookies, yt-dlp (2026.06.09) uses YouTube's authenticated web client, whose
formats need a JS runtime for the n-challenge — Finder PATH has no node/deno
(they live in `~/.nvm`/`~/.deno`), so yt-dlp reported "n challenge solving
failed … Only images are available" → `ERROR: Requested format is not
available` at `--dump-json` time; (3) `make_error_user_friendly` matched
"unavailable" but not "not available" (and "timeout" but not "timed out"), so
the real error flattened to the generic fallback. **Fix (two halves):** the
.app build (`scripts/build-macos-app.sh`, PR #55 branch) now bundles Deno
into `Contents/Resources/bin/` — the launcher's PATH-prepend makes it visible
to both yt-dlp and `depcheck::has_js_runtime()`, completing the self-contained
bundle (yt-dlp + ffmpeg/ffprobe + JS runtime); and `error.rs` adds a
format-specific arm ("format" + "not available" → a no-downloadable-formats
message naming the JS-runtime/cookies angle, checked before the generic
not-available arm) plus "timed out" alongside "timeout" and plain "not
available" alongside "unavailable", unit-tested. Verified against the exact
failing condition: bundled yt-dlp + `--cookies-from-browser chrome` +
Finder-equivalent PATH fails at `37d2a7d` and succeeds with only deno added.
Source: maintainer report + diagnosis session, 2026-07-04.

### B-GUI-005 — "Format" control was a dead static "MP4" label · closed (PR open) · MEDIUM
The main view's Format tag was hardcoded text (`main_view.rs`, "static for
now"); nothing about container/format reached the download, so files kept
yt-dlp's native container — YouTube "Best" delivered AV1/Opus in `.webm`
under an "MP4" label. **Fix:** a real `OutputFormat` selector
(`utils/config.rs`): `Best` (default, "Original (Best)" — byte-identical
no-conversion behaviour), video containers MP4/WebM/MKV mapping to yt-dlp
`--remux-video <ext>` (lossless container change, flag verified against the
bundled 2026.06.09), and audio MP3/M4A/Opus/FLAC/WAV mapping to the engine's
existing `-x --audio-format` path. GUI: a `pick_list` mirroring the Quality
selector, persisted in the settings table (`output_format` key; unknown
values load as Best), plus a hint line stating the trade-off (remux is
lossless; an impossible remux keeps the original container). Threading:
`StartDownload` → `DownloadTask` → `QueueEvent::TaskAdded`
(`#[serde(default)]`, so pre-existing event-log lines rehydrate as Best —
I-6 untouched) → `download_with_fallback(output_format)`. A non-Best choice
routes via the page URL (the B-GUI-004 move) so the yt-dlp path performs the
ffmpeg post-step — the native path can't convert. Remux failures
(Postprocessing error + file on disk) keep the original container instead of
failing the download; audio post-failures still fail. The engine-shared
`YtDlpOptions` is not mutated: the choice applies per invocation in
`download_via_ytdlp` (Arc-shared engine, same rule as `format_spec`). CLI
surface unchanged. Unit tests: label/key round-trip, one-action-per-variant,
`--remux-video` emission (suppressed under `-x`), remux-failure keep-original
(stub yt-dlp), non-remux post-failure still fails. Known limitation, stated
in the UI hint: "MP4 + Best" on YouTube remuxes the AV1/VP9 best streams
into MP4 rather than filtering to native-H.264 MP4 (which caps at ~1080p);
WebM cannot hold H.264 sources, which is exactly the keep-original case.
Source: maintainer report (webm file under an "MP4" label), 2026-07-05.

## P2

### F-DL-001 — Shape A: use aria2c as yt-dlp's external downloader · closed (opt-in) · SMALL (XS)
Add `--downloader aria2c` to `build_ytdlp_args` when an **external** `aria2c` is
detected (mirror `find_ytdlp` detection; do NOT bundle — aria2 is GPL-2.0, see
`adr/0002`). Gated on B-DOC-001 (license posture); landed after it.
Source: internal audit 2026-06-30.

### B-DL-010 — Engine flattened yt-dlp download failures to "yt-dlp download failed" · closed (PR open) · SMALL
Found during B-DL-009 verification (B-DL-009 is tracked on the PR #56
branch): `download_via_ytdlp` (`engine.rs`) returned
`Err(anyhow!("yt-dlp download failed"))` on a non-zero yt-dlp exit,
discarding the `ERROR:` stderr line its own reader task had already captured,
logged, and sent as a `Failed` progress event. Downstream the CLI runs
`make_error_user_friendly` on that flattened string (`cli.rs`, B-DL-007), so
a download-stage yt-dlp failure could only ever surface as the generic
"Unable to process this URL" — the detailed patterns added in B-DL-009 never
saw the real text. **Fix:** the stderr reader task now returns the detected
`ERROR:` line; the failure arm carries it in both the terminal `Failed`
progress event (I-3 contract unchanged — same event, better detail) and the
returned `Err`, falling back to "yt-dlp download failed (no error output
captured)" when yt-dlp exits non-zero without printing one. Test seam: the
spawned program name is a `DownloadEngine` field (`"yt-dlp"` everywhere;
`#[cfg(test)]` override), so the unix-gated regression test
`test_ytdlp_failure_carries_stderr_error_line` drives the real process path
with a stub script — no PATH mutation. Source: B-DL-009 fix session,
2026-07-05.

**Correction (2026-07-01 implementation) — the progress-contract assumption
above was wrong, verified empirically, not assumed:** live-smoke-tested
`yt-dlp --downloader aria2c` against both an HLS stream and a direct HTTP file,
and read yt-dlp's own `downloader/external.py` source. Two findings:
1. **aria2c never engages for HLS/DASH at all** —
   `Aria2cFD.SUPPORTED_PROTOCOLS = ('http', 'https', 'ftp', 'ftps')` in yt-dlp's
   source; `m3u8`/`dash` aren't in that list, so yt-dlp silently falls back to
   its native `hlsnative` downloader regardless of the flag. Confirmed live: a
   `--downloader aria2c` run against an HLS test stream produced
   byte-identical `[hlsnative]`-tagged log output to a run without the flag —
   i.e. **zero benefit for `download_via_ytdlp`'s primary use case** (HLS/DASH
   fallback).
2. **Where aria2c *does* engage (plain http/https/ftp), progress breaks.**
   yt-dlp's `ExternalFD.real_download()` calls its progress hook exactly
   **once, on completion** — not incrementally during the transfer. Live-smoke
   confirmed: a direct-HTTP-file run with `--downloader aria2c` printed
   aria2c's own raw progress format (`[#10ff12 ...]`) throughout the transfer,
   and only the very last line matched `parse_yt_dlp_progress`'s expected
   `[download] X% of Y at Z` shape (the 100%-at-completion line). The
   intermediate aria2c-format lines partially misparse (a coincidental digits-
   before-`%` match extracts a plausible-looking percentage, but `total_bytes`
   parses to `0`, which `DownloadProgress::percentage()` clamps to `0%`) — net
   effect: the progress bar sits frozen at 0% for the whole transfer, then
   jumps to 100%. This is a real I-3 regression for the one case the flag
   would actually change anything.

**Fix landed (PR [#31](https://github.com/ibra2000sd/rustloader/pull/31),
open, not yet merged):** implemented as **opt-in, default off**
(`YtDlpOptions::use_aria2c: bool`, defaults `false` via `derive(Default)`).
`find_aria2c()` (`extractor/ytdlp.rs`, mirrors `find_ytdlp`/
`find_in_common_paths`, deliberately **without** `find_ytdlp`'s
bundled/adjacent-to-executable check — I-9/ADR-0002) detects an external
aria2c; `build_ytdlp_args` takes the caller's already-resolved
`aria2c_available: bool` and only then adds `--downloader aria2c`. No CLI/GUI
toggle is wired up in this PR (nothing currently sets `use_aria2c: true`), so
this PR changes nothing by default — verified via `cargo run -- <url>
--dry-run`, byte-identical args to before. Exposing it as a real opt-in
(CLI flag or GUI setting) is a follow-up, once/if the progress-hook gap above
is separately addressed (or accepted as a documented tradeoff for that
narrow case).

**F-DL-001b — enable path (PR
[#33](https://github.com/ibra2000sd/rustloader/pull/33), open, not yet
merged):** added the actual opt-in: `Cli::experimental_aria2c` (CLI flag
`--experimental-aria2c`, default `false`), threaded to
`YtDlpOptions::use_aria2c` in `to_ytdlp_options()`. Help text labels it
`EXPERIMENTAL` up front and states the progress-freeze caveat from the
correction above verbatim, so turning it on is a conscious, informed choice,
not a casual toggle. No GUI checkbox added — `gui/app.rs` has no existing
settings-UI widgets (checkboxes/toggles) to attach a "advanced/experimental"
option to today, and the task explicitly discouraged a prominent toggle;
adding one properly (its own advanced-settings area, non-prominent, labelled)
would be a materially larger change than "wire the CLI flag." Left as an
explicit follow-up. Absent the flag, `build_ytdlp_args`'s output is verified
byte-identical to before (`cargo run -- <url> --dry-run`, no `--downloader`
in the args); with the flag and a real `aria2c` installed, the same command
shows `--downloader aria2c`.

### F-DL-003 — Byte-level resume + checkpoint persistence (native engine) · closed (Shape 2) · MEDIUM-LARGE
**Correction (2026-07-01 spike):** the previous framing of this item — that the
`download_segments` SQLite table is "already written/read in
`database/operations.rs`" and that resume is therefore largely a DB wire-up —
was stale/inaccurate. A read-only spike verified `save_segment`/`get_segments`/
`save_download`/`get_download`/`get_all_downloads`/`get_downloads_by_status`/
`delete_download` have **zero callers anywhere outside `database/operations.rs`
itself**; the table is fully dead, not partially wired. The spike also found
that #28/#29 already made cross-session resume *happen* as an unintentional
side effect (deterministic `calculate_segments`, no `.partN` cleanup on pause/
cancel/app-close, `output_path`/URL preserved via the in-memory task or the
`EventLog`) — but with **zero validation** that the on-disk parts belong to the
current plan, which is a latent silent-corruption bug, not a missing feature.
See the spike report (session transcript, 2026-07-01) for full evidence.

**Fix landed (Shape 2 — sidecar identity guard, PR
[#30](https://github.com/ibra2000sd/rustloader/pull/30), merged `f51dfad`,
2026-07-01):** a small `<output>.rustloader-resume` sidecar records
`{url_hash, file_size, segment_count}` before segment downloads start. On
every `download()` call, an existing `.partN` set is only trusted if the
sidecar matches the current identity *and* `enable_resume` is `true`;
otherwise (mismatch, missing sidecar with parts present, or resume disabled)
the parts are discarded and the segment loop starts clean. This closes the
two corruption paths: a segment-count preference change between sessions, and
a different download reusing the same `output_path`. `enable_resume` finally
gates real behavior instead of being a dead, always-on flag.

**Deliberately NOT done here (spinoffs, separate items):**
- **Orphaned `.partN` cleanup on cancel/remove** — `pause_task`/`cancel_task`/
  `remove_task` (`queue/manager.rs`) never call `cleanup_segments`, so
  cancelled downloads leave parts on disk indefinitely. The identity guard
  makes this *safe* (a mismatch/foreign check would clean them up on the next
  attempt at that path), but the litter itself is unaddressed. **Filed and
  done as `B-DL-002` below.**
- **Shape 3 / DB-backed persistence** — using `downloads`/`download_segments`
  to store the plan instead of (or in addition to) the filesystem sidecar
  remains a legitimate future direction (would also unlock download history/
  resume across a moved output path), but is out of scope for this fix. The
  `downloads` half (history, not resume) is now `F-HIST-001` below;
  `download_segments`-backed resume remains unaddressed and out of scope
  there too — the sidecar still owns resume.
Source: internal audit 2026-06-30; F-DL-003 spike 2026-07-01.

### B-DL-002 — Orphaned `.partN` + resume sidecar left behind on cancel/remove · closed (PR open) · SMALL
The F-DL-003 hygiene spinoff (see the "Deliberately NOT done" list above):
`cleanup_segments` ran only after a successful merge (`engine.rs`), so
`cancel_task`/`remove_task` left the `.partN` files and the
`<output>.rustloader-resume` sidecar on disk indefinitely — safe (the sidecar
identity guard prevents corruption from stale parts) but litter. **Fix (PR
[#36](https://github.com/ibra2000sd/rustloader/pull/36), open, not yet
merged):** a private best-effort `cleanup_task_artifacts` helper in
`queue/manager.rs` removes the sidecar (via `resume_guard`'s
`sidecar_path`/`remove_sidecar`) and every `<file_name>.part<digits>` file in
the output's directory (a strict digit-suffix scan — the segment count isn't
known at the queue layer). Called from `cancel_task` and `remove_task` only,
after the locks are released; failures are logged, never propagated.
`pause_task` deliberately does NOT clean up — parts + sidecar must survive
pause or cross-session resume (F-DL-003/#30) breaks; `clear_completed` needs
nothing because the engine already removes both at merge time
(`engine.rs`). Regression tests in `tests/orphan_cleanup_test.rs`: cancel
and remove delete the artifacts (decoys untouched), and — the load-bearing
guard — pause keeps them.
Source: F-DL-003 spinoff, 2026-07-01; implemented 2026-07-02.

### F-HIST-001 — Shape-3 PR-1: persist download history to the `downloads` table · closed (headless) · MEDIUM-LARGE
The `downloads` table (and its CRUD — `save_download`/`get_download`/
`get_all_downloads`/`get_downloads_by_status`/`delete_download`) has been dead
since it was first defined — zero callers anywhere outside
`database/operations.rs` itself (confirmed by the F-DL-003 spike, re-confirmed
at this item's own HEAD). This wires it into the live download lifecycle as a
durable **history** (survives an app restart), separate from — and not a
replacement for — the sidecar-based resume mechanism (#30) or the EventLog's
live-queue rehydrate.

**Design decisions (see PR
[#34](https://github.com/ibra2000sd/rustloader/pull/34) description for full
detail):**
- **Injection:** `gui/app.rs` already builds one `DatabaseManager` (used today
  only for the `settings` table); `BackendActor::new` now takes an
  `Arc<DatabaseManager>` — the SAME instance, cloned via `Arc::clone`, not a
  second pool/file — and stores it. `QueueManager` itself is untouched.
- **Identity:** `downloads.id` = the queue task ID
  (`Uuid::new_v4().to_string()`, generated in `handle_start_download`) — the
  same ID `EventLog`/`DownloadTask.id` already use, so a history row and a
  live queue task reconcile 1:1 by construction.
- **EventLog coherence:** `QueueManager`/`EventLog` remain the sole runtime
  authority for a task's live state (I-4 unchanged — `queue/manager.rs` and
  `queue/events.rs` have zero diff in this PR). The `downloads` table is a
  best-effort, derived, write-only *projection* of that authority: one row is
  inserted (status `Queued`) right after a successful `queue_manager.add_task`
  in `handle_start_download`, and updated (same row, `INSERT OR REPLACE`
  keyed by id) on every subsequent status transition, detected by the
  existing `monitor_loop` polling diff (`backend/actor.rs`) that already
  drove the GUI's `TaskStatusUpdated` event — no new detection mechanism, no
  second lock/authority. Removing a task from the live queue (cancel/remove)
  does **not** delete its history row — history is meant to outlive the live
  queue entry, that's the point of a persistent history.
- **`download_segments` stays dead** — not wired, not touched. Resume is
  still exclusively the `.rustloader-resume` sidecar's job.

**Tests:** `database/operations.rs` gained two regression tests —
`download_history_survives_reopening_the_database` (writes rows, drops the
pool, opens a brand-new pool against the same file, reads them back via
`get_all_downloads` with correct status/fields — the literal "survives a
simulated restart" acceptance bar) and
`status_transitions_update_in_place_not_duplicate` (three transitions of one
task id via `save_download` leave exactly one row, not three, with the final
status/`completed_at` and the original `created_at` preserved). Two pure unit
tests for the new `task_status_db_fields` mapping helper (in
`backend/actor.rs`) cover every `TaskStatus` variant, including that only
terminal states set `completed_at` and that `Failed`'s message carries
through.

**Headless in this PR — no GUI history list** (that's `F-HIST-002` below);
`BackendActor::download_history()` is a plain accessor + a startup log line
proving the data is live and durable, with nothing rendering it yet.

### F-HIST-002 — GUI download-history list (Shape-3 PR-2) · closed · MEDIUM
Render download history (`get_all_downloads()`/`delete_download()`, added by
`F-HIST-001`) in the GUI — a history view separate from the live queue list,
showing past downloads (including ones cleared from the active queue) with
their final status.

**Fix (PR [#35](https://github.com/ibra2000sd/rustloader/pull/35), open, not
yet merged):**
- **New `View::History`** — a third sidebar entry (alongside Downloads/
  Settings), matching the existing `settings_view`-style free-function view
  pattern (`src/gui/views/history_view.rs`, `src/gui/components/
  history_item.rs`). Lists persisted downloads newest-first (title, falling
  back to URL when empty; status; output path; file size or an explicit
  "Unknown size"; the completed/created timestamp), with a distinct
  loading/error/empty state.
- **Data access — direct, not routed through `BackendActor`:** `gui/app.rs`
  already holds the exact same `Arc<DatabaseManager>` the actor uses (#34);
  the History view reads/writes it directly via `Command::perform`, the same
  async pattern already established for Settings save/load. A
  `BackendCommand::GetHistory`/`BackendEvent::History` round-trip through the
  actor's channel would only add indirection with no benefit, since the GUI
  can already reach the same database the actor does.
- **Remove from history** deletes the DB record only (`delete_download`) —
  never the downloaded file — with an explicit label to that effect; optimistic
  local removal, reconciled by a reload on failure.
- **Show in Folder** reuses the exact `open::that(...)` call already shipped
  for the live queue's `OpenDownloadFolder` (`open` crate, already a
  dependency — no new one added); same detached, non-blocking, fire-and-forget
  behavior, just pointed at a `DownloadRecord`'s `output_path` instead of an
  active `DownloadTaskUI`'s.
- **Auto-refresh:** the existing status-diff logic in `BackendActor`'s
  `monitor_loop` already emits `DownloadCompleted`/`DownloadFailed`/
  `TaskStatusUpdated`; when one of those signals a terminal state AND the
  History view is the one currently open, the GUI reloads history — so a
  visible history list doesn't go stale while the user is looking at it,
  without polling.
- Re-download / open-file from history are explicit fast-follows, not in this
  PR.

No persistence/engine/queue/sidecar changes — `src/queue`/`src/downloader`
have zero diff; `database/operations.rs`'s only change is new tests (no new
or modified CRUD methods — `get_all_downloads`/`delete_download` already
existed from `F-HIST-001`). `download_segments` remains untouched/dead.

### B-DOC-002 — KNOWN_ISSUES.md content is stale · closed · SMALL
`B-DOC-001` fixed only the title's version stamp (now "v0.8.1"); the body was
still written around v0.1.1 and predated the whole download-reliability arc
(#28/#29/#30/#31). **Fix (PR [#32](https://github.com/ibra2000sd/rustloader/pull/32),
open, not yet merged):** rewrote the body against verified current-HEAD
evidence — resume scope (segmented-only, sidecar-guarded per F-DL-003;
`download_simple` and the yt-dlp/HLS path still don't resume), orphaned
`.partN` files on cancel (the F-DL-003 spinoff), aria2c's opt-in/not-yet-exposed
status (F-DL-001), the real cross-platform CI/release picture (builds+tests
pass on all 3 OSes; Windows/Linux release still "planned" per ROADMAP), and a
re-verified dependency-warning list (`cargo audit`: `instant`/`paste`/
`ttf-parser` unmaintained, `lru`/`memmap2` unsound, all transitive via `iced`).
Dropped two claims that no longer held: "82→15-20 compiler warnings" (a fresh
`cargo build` shows zero warnings from the crate's own code today) and
"limited unit test coverage" (186 tests pass across unit/integration/
persistence/stress suites). Added a "Recently Resolved (v0.8.x)" table for the
F-DL-002/B-DL-001/F-DL-003 fixes. Docs-only; no Rust source touched.
Source: internal audit 2026-06-30.

## P3 / later

### F-EXTRACT-001 — Proxy-capture spike (res-downloader style) · open · investigate-first
Exploratory spike for a local-proxy media capture ("any page that plays video",
no extractor). Heavy: requires a system proxy + a local CA certificate in the
trust store (real security/UX surface). **Scope boundary (mandatory):** legitimate
capture only (public content without an extractor, content the user owns, m3u8 the
browser already plays); explicitly **excludes** defeating DRM/content protection.
Reference upstream `putyy/res-downloader` is Apache-2.0 (Go+Wails — port the
approach, not the code). Relates to the CHANGELOG's "Browser extension
integration (v1.0.0)" plan. Decision needed from maintainer before drafting.

**Phase-0 (desk) — COMPLETE, 2026-07-02 (base `9eaee55`):** MITM base = `hudsucker`
(MIT/Apache); `slinger-mitm` **rejected** (GPL-3.0-only, ADR 0002 / I-9); `rcgen`
(MIT/Apache, ECDSA P-256) for the per-install CA. Machine-level prototype (root-CA
install + system-proxy + real-site pinning) **pending a supervised session** — not
run unattended, not fabricated. Findings:
[`docs/ai-os/spikes/F-EXTRACT-001-phase0-findings.md`](spikes/F-EXTRACT-001-phase0-findings.md);
decision record: [`adr/0004-proxy-capture-mitm.md`](adr/0004-proxy-capture-mitm.md)
(Proposed, gated). Entry stays **open** — Phase-0 records go/no-go only.

### B-DL-003 (optional) — Reconsider the 1800s yt-dlp download timeout · open · SMALL
`download_via_ytdlp` is correctly bounded but 30 min is generous; consider
lowering / making it configurable. NOT a bug (it is already bounded).

### F-GUI-001 — Opt-in clipboard monitoring (detect copied URLs) · closed (PR open) · SMALL-MEDIUM
The cheap, in-app half of "capture what you copy" (the browser extension +
receiving endpoint is a separate track). A Settings toggle (**default OFF**,
plainly labelled — monitoring the clipboard is privacy-sensitive) enables a
2-second `iced::time::every` poll subscription; a newly copied single-token
http(s) URL surfaces a confirm/dismiss banner on the Downloads view, and
**confirming** queues it through the existing `ExtractInfo` add path (I-2) —
nothing ever auto-downloads. De-dup via a `ClipboardWatch` last-seen tracker
(same value never re-prompts; first observation after enabling only seeds, so
pre-existing clipboard content is ignored; the app's own Paste marks content
seen so it isn't re-offered). Non-URL clipboard content is ignored silently;
clipboard text is never stored, logged, or transmitted. Pure detection/de-dup
helpers live in `src/gui/clipboard_monitor.rs` with unit tests; the
`clipboard_monitoring` flag persists via the existing settings table. GUI-only
— no engine/resume/persistence change (I-3 untouched). 2026-07-02, base
`e8ebbe1`, PR pending.

### F-GUI-002 — Design-system foundation: theme tokens + fonts (phase 1) · closed (PR open) · MEDIUM
The maintainer's Claude-Design kit (`Rustloader_Design_System.zip`) committed
as `design-system/` (brand source of truth) and its Iced-achievable subset
applied: `src/gui/theme.rs` rewritten to the exact `tokens/colors.css` dark
palette (surfaces `#0A0908→#23201B`, rust accent `#CF6F38`, amber `#E9B44C`
reserved for live data, warm status colors), radius scale (4/8/10), and the
readme's documented fallbacks (glass → solid `--bg-2`, no gradients, default
motion). Geist + Geist Mono bundled (OFL-1.1, vercel/geist-font v1.7.2,
license at `assets/fonts/OFL.txt`), registered via `Settings::fonts`; all
data text (speeds, sizes, ETAs, counts, paths, timestamps, URLs) renders in
Geist Mono. Built-in widgets follow via `Theme::custom` palette mapping. The
window icon was already wired at HEAD (verified byte-identical to the kit's
PNGs — not re-done). GUI-only; no engine/queue/persistence change.
2026-07-03, base `1a08571`, PR pending.

### F-GUI-003 — Design-system per-component restyling (phases 2+) · open · MEDIUM-LARGE
Follow-ups after the F-GUI-002 foundation, per `design-system/readme.md`,
each a separate PR: (1) component fidelity — `DownloadItem` states, the
signature **`SegmentBar`** (N-cell parallel-segment progress), `UrlBar`,
`HistoryItem`, `ClipboardBanner`, stat bento tiles, Lucide icons replacing
unicode glyphs; (2) the `[data-theme="light"]` light theme; (3) the design
brief's fixed 1080×720 non-resizable window (a behavior change — maintainer
call); (4) a Windows `.ico` if/when Windows bundle packaging exists (macOS
`AppIcon.icns` already ships; no Windows packaging is in the repo today).
A full-fidelity web-shell (Tauri) migration is a separate strategic decision.

### `B-DL-007` — native download output handling (naming + missing dir)

Two pre-ship smoke findings in the native download path, both in
`src/downloader/engine.rs` (+ a truthful-error tweak in `src/cli.rs`):

1. **Unknown / `application/octet-stream` content was saved as `<name>.mp4`.** The
   B-DL-006 extension map has no octet-stream entry, so it fell through to the
   audio/video *mode default* (`.mp4`) — a checksum file from a GitHub release
   asset landed as `<uuid>.mp4`. Now the final name is derived: media
   `Content-Type` → correct ext (unchanged); else URL-path ext; else the
   **`Content-Disposition`** filename; else the URL basename; else `.bin`. Never
   `.mp4` for a non-media binary, and the server-provided name replaces bare-UUID
   names. B-DL-006 media behaviour and #37 temp-rename are preserved.
2. **A missing `-o` directory failed ungracefully** — every segment errored
   `No such file or directory` and the user saw the misleading `Unable to process
   this URL`. The engine now `create_dir_all`s the output parent up front, and
   `cli.rs` surfaces genuine filesystem errors verbatim instead of the generic
   URL message.

Regression tests cover the extension-derivation matrix (incl. Content-Disposition
parsing + octet-stream→`.bin`) and the directory creation. 2026-07-02, base
`68c0ee0`, PR pending.

## Recently closed

| ID | Title | Closed by |
|----|-------|-----------|
| — | Tolerate float `duration` in `VideoInfo` deserialize | PR #21 (`dff16f2`) |
| — | Robust default yt-dlp format selector (HLS master) | PR #22 (`933b2c0`) |
| — | Bound yt-dlp **extraction** subprocess with timeout + kill | PR #23 (`1c038e2`) |
| `B-DOC-001` | README/LICENSE/roadmap claims are inaccurate | `f897872`, 2026-07-01 (PR pending) |
| `F-DL-002` | Segment retry resumes from written bytes (retry-resume half) | PR #28, `c1c0580`, 2026-07-01 |
| `B-DL-001` | Segment resume requires HTTP 206, else restarts | PR #29, `c976872`, 2026-07-01 |
| `F-DL-003` | Cross-session resume sidecar identity guard | PR #30, `f51dfad`, 2026-07-01 |
| `F-DL-001` | Opt-in aria2c external downloader for yt-dlp path | PR #31, `38ea148`, 2026-07-01 |
| `B-DOC-002` | KNOWN_ISSUES.md content refresh | PR #32, 2026-07-01 (PR pending) |
| `B-DL-006` | Saved extension reflects actual content, not the mode flag | 2026-07-02 (PR pending) |
| `F-GUI-001` | Opt-in clipboard monitoring (detect copied URLs, confirm to queue) | 2026-07-02 (PR pending) |
| `B-DL-007` | Native downloads: create missing output dir; name/ext for unknown content (Content-Disposition → URL basename → `.bin`, never `.mp4`) | 2026-07-02 (PR pending) |
| `F-GUI-002` | Design-system foundation: token palette + Geist/Geist Mono in the Iced theme; `design-system/` committed | 2026-07-03 (PR pending) |
| `B-DL-008` | Engine yt-dlp fallback runs on the original page URL + chosen-format spec, not the dead direct URL | 2026-07-03 (PR pending) |

(Pre-`docs/ai-os` work was tracked via GitHub PRs/CHANGELOG; future items use the
IDs above.)
