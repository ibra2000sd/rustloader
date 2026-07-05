# Browser integration — design (extension → rustloader)

Status: **proposed** (design spike, no implementation yet)
Date: 2026-07-05 · Verified against `main` @ `f6de70a`
Scope: how an IDM-style browser extension hands a URL (+ page cookies/headers +
chosen quality/format) to rustloader. Chrome first; Firefox/Edge later.
Explicitly **not** MITM/proxy interception — an extension plus a local bridge.

---

## 1. The integration surface today (verified)

How rustloader accepts a URL at `f6de70a`:

| Path | Mechanism | Programmatic? |
|---|---|---|
| CLI | `rustloader <url>` → `cli::run()` does a headless download in a **new process** and exits (`src/main.rs:27`, `src/cli.rs:257`) | Yes, but it never talks to a running GUI |
| GUI text field | `Message::UrlInputChanged` → `BackendCommand::ExtractInfo` to the `BackendActor` (`src/backend/messages.rs:8`) | No |
| Clipboard monitor | Opt-in iced subscription polls every 2 s, `detect_url()` surfaces a copied http(s) URL (`src/gui/clipboard_monitor.rs`) | Indirectly (copy = enqueue prompt) |

**Single-instance behaviour: there is none.** No IPC, no local socket, no
server, no URL-scheme registration exist anywhere in the tree (all
`TcpListener` hits are test-only mock servers in `segment.rs`,
`engine.rs`, `update_check.rs`). A second invocation is a fully independent
process: `rustloader <url>` downloads headlessly on its own; a second GUI
launch opens a second window. Nothing can hand a URL to a *running*
instance today — **the bridge is a new capability, not a reuse of an
existing one.**

**Cookies today:** one type, `CookieConfig` (`src/utils/cookies.rs`), carrying
either `--cookies-from-browser <name>` or `--cookies <netscape-file>`, applied
identically to every yt-dlp call (invariant **I-7**). The extension path slots
in cleanly: extension-supplied cookies are written to a Netscape-format temp
file and passed as `CookieConfig { file }` — no new flag assembly anywhere.
This is also a *fix*, not just a feature: `--cookies-from-browser chrome` on a
packaged macOS build needs Keychain access via `/usr/bin/security` and has
been a real failure source; cookies pushed by the extension bypass Keychain
entirely.

**Useful existing scaffolding:**
- The clipboard monitor is the exact template for an opt-in "external URL
  source" subscription feeding GUI messages.
- `BackendCommand`/`BackendEvent` channels (invariant **I-2**: the GUI is the
  only driver of the actor) tell us where a bridge must inject work: into the
  **GUI message loop**, not directly into the engine.
- Settings persist as key-value rows in SQLite
  (`load_settings_from_db`, `src/gui/app.rs`) — a bridge toggle, port, and
  pairing token fit the existing pattern.
- `hyper` 1.x is already compiled into the tree (via `reqwest 0.12`), so an
  HTTP server adds near-zero new dependency surface.

**License:** rustloader is **MIT** (`Cargo.toml`, `LICENSE`). Consequence: we
may *learn from* GPL-3.0 projects but must not copy their code.

---

## 2. OSS references studied (licenses noted)

All five repos were fetched and read on 2026-07-05 (GitHub API + raw files).
rustloader is MIT, so: **MIT sources may be adapted with attribution; GPL-3.0
and unlicensed sources are pattern-reference only — no code reuse.**

| Repo | License | Bridge mechanism | What we take (patterns only where required) |
|---|---|---|---|
| `tonhowtf/omniget` | **GPL-3.0** — learn-only | Loopback HTTP (axum) in the app + bearer token; `omniget://` scheme fallback (URL only, no cookies) | The architecture we recommend. Notably its code comments state it **migrated away from native messaging** because it "requires hard-coded extension IDs in the desktop app" |
| `imsyy/yt-dlp-gui` | MIT | Pure `ytdlp-gui://` deep link; cookies as base64 Netscape text **inside the URL**; `tauri-plugin-single-instance` forwards second-launch args | The Netscape-serialization idea and the single-instance-forwarding shape — and a cautionary example: no token, no origin check, session cookies travelling through an OS-level URL launch |
| `opalsaints/yt-dlp-chrome-extension` | MIT | Native messaging, Python host, macOS; `sendNativeMessage` one-shots | The PATH lesson: Chrome launches hosts without a login shell, so the installer wraps the host in a PATH-exporting script (rustloader already fights this exact problem in `setup_bundled_tools_path`) |
| `exotic123567/yt-dlp-bridge` | **none** (all rights reserved) — learn-only | Native messaging, Python host, Windows registry (.reg) registration for Chrome + Brave | Illustrates the per-browser registry/manifest cost of option A |
| `ajnewlands/chrome-ext` | **none** (all rights reserved) — learn-only | Native-messaging host in Rust (2020, tokio 0.2): Chrome's 4-byte **native-endian** length-prefixed stdio framing via `LengthDelimitedCodec` | If we ever do add a native-messaging host, the framing is one `tokio_util` codec — but see §3 |

Details worth recording from omniget (the closest analog — patterns, not code):
- Port discovery: fixed 10-port range (47720–47729), extension probes all in
  parallel with `GET /v1/health` (1.5 s timeout), first responder wins; the
  app persists the port it bound.
- Token: 32 random bytes, URL-safe base64, **constant-time comparison**;
  CORS is deliberately `Any` with the comment "the token is the actual auth
  boundary".
- **Pairing window:** clicking "Pair" in the desktop app opens a single-use
  ~120 s window during which `GET /v1/pair` hands the token to the extension
  — user-consented auto-pairing with almost no code (an atomic deadline +
  one endpoint).
- Cookie wire format: JSON array of `{name, value, domain, path, secure,
  httpOnly, expires, sameSite}` records inside the enqueue payload.
- Its MV3 sniffer is observe-only `webRequest` (`onSendHeaders` /
  `onHeadersReceived`) with a content-type allowlist + extension list,
  then aggressive noise filters (analytics paths, HLS `.ts`/`seg-N`
  patterns, small CDN fragments).
- Scheme fallback carries the **URL only** — cookies never go through the
  OS URL launch (unlike yt-dlp-gui). We adopt the same rule.

---

## 3. Bridge architecture: native messaging (A) vs localhost HTTP (B)

### A — Native messaging
Browser ↔ native host over stdio (4-byte-length-prefixed JSON). The host is
registered per browser per OS (registry key on Windows; JSON manifests in
browser-specific directories on macOS/Linux) with an `allowed_origins`
whitelist of extension IDs.

### B — Localhost HTTP server in rustloader + pairing token (+ optional `rustloader://` fallback)
The running app binds `127.0.0.1:<port>`, the extension POSTs JSON to it with
a bearer token the user pairs once via the extension options page.

### Comparison

| Criterion | A: native messaging | B: localhost HTTP + token |
|---|---|---|
| Security baseline | Browser-mediated; no open port; `allowed_origins` pins the extension ID | Open loopback port; must self-defend: loopback-only bind + random token + Host/Origin checks |
| Cross-browser cost | Per-browser + per-OS host manifests (Chrome/Edge/Firefox each different paths; Windows = registry) | One server serves every browser; only the extension client varies |
| **Fit with rustloader's shape** | **The killer problem:** the host Chrome spawns is a *separate process* from the running GUI. rustloader has **no IPC** for that host to forward into — so A requires building a local socket/server *anyway*, plus the host binary, plus manifest registration. A ⊇ B in implementation cost. | The server lives inside the already-running GUI process as a tokio task feeding the iced message loop — the same shape as the clipboard monitor |
| App lifecycle | `connectNative` auto-launches the *host*, but launching the full GUI from a headless host is awkward (and on macOS the bundle context differs) | App not running → connection refused → extension shows "launch rustloader" (Phase 1); `rustloader://` scheme can cover auto-launch later |
| Install/pairing UX | Zero pairing, but installer must write host manifests/registry keys per browser | One-time token paste (or auto-pair confirm dialog); no installer changes |
| Precedent | ajnewlands/chrome-ext (Rust host), Python bridge hosts | omniget, yt-dlp-gui — the modern Rust/Go download-manager pattern |

### Recommendation: **B — localhost HTTP bridge inside the app, loopback-only, token-paired**

Rationale, in order of weight:

1. **A collapses into B on this codebase.** Because rustloader has no
   existing IPC, a native-messaging host would still need a socket/channel
   into the running app. Native messaging here means building *both*
   mechanisms; the stdio host adds nothing we need.
2. **One bridge, N browsers.** Chrome/Edge/Firefox/Brave all speak
   `fetch()` to localhost; only manifest packaging differs per browser.
3. **It matches the app's architecture.** A tokio task + mpsc into an iced
   subscription is exactly how backend events and the clipboard monitor
   already flow (invariant I-2 stays intact: the bridge feeds the GUI, the
   GUI drives the actor).
4. The security gap vs A is real but well-understood and closable
   (§6): loopback bind, 128-bit token required on every request, `Host`
   header check (DNS-rebinding), no wildcard CORS, token never logged.
5. **The closest working analog made the same call.** omniget's code
   comments (in both its extension bridge client and its Rust server)
   record a deliberate migration *from* native messaging *to* the loopback
   bridge, because native messaging hard-codes extension IDs into the
   desktop app and multiplies install surface per browser.
6. Native messaging remains available as a *future additive option* (e.g.
   for a browser that blocks localhost fetches); nothing in B forecloses A.

The `rustloader://` scheme is **deferred to a late phase**, not Phase 1:
iced 0.12 pins winit 0.29, and winit through 0.30 does not implement
`application:openURLs:` on macOS — a scheme handler there means registering a
`kAEGetURL` Apple-event handler by hand (objc2) or waiting for winit 0.31+
(rust-windowing/winit#2190). Windows/Linux are easy (registry / `.desktop`
`%u`), but shipping a scheme that works everywhere *except* the maintainer's
primary platform is the wrong first slice. Phase 1's answer to "app not
running" is honest UX: the extension detects connection-refused and tells the
user to launch rustloader.

---

## 4. Extension architecture (Chrome, MV3)

Verified against current Chrome docs (2026-07): MV3 allows **observational**
`webRequest` with the `webRequest` permission + matching `host_permissions`;
*blocking* webRequest is enterprise-policy-only, and we don't need it.
`chrome.cookies.getAll({url})` needs the `cookies` permission + host
permissions.

```
extension/
  manifest.json          MV3; permissions: contextMenus, cookies, storage, notifications
                         host_permissions: http://127.0.0.1/* (+ <all_urls> once the
                         sniffer/cookies need it; scoped tighter if store review pushes back)
  background.js          service worker: context-menu registration + click handler,
                         bridge client, badge state
  bridge-client.js       POST /api/v1/download with token; port discovery via
                         GET /api/v1/ping over the port range; cached port in storage.local
  cookies.js             chrome.cookies.getAll({url: pageUrl}) → protocol cookie array
  sniffer.js             (Phase 2) webRequest observe-only: media content-types
                         (video/*, audio/*, application/x-mpegURL, application/dash+xml)
                         and extensions (.m3u8, .mpd, .mp4, .webm); drop segment noise
                         (.ts, .m4s, init/…, /seg-…); per-tab detected list; action badge
  popup.html/js          (Phase 2) detected media + quality/format choice
  options.html/js        pairing page: token entry (or auto-pair), status indicator
```

Phase 1 uses only: manifest, background service worker, bridge client,
cookies module, options page, and a single context-menu item —
**"Download with Rustloader"** on `page`, `link`, and `video`/`audio`
contexts — sending the page/link URL.

---

## 5. Bridge protocol (v1)

Transport: HTTP/1.1 on `127.0.0.1:<port>`, port chosen from a fixed scan
range **46150–46159** (first free wins; extension discovers by pinging the
range). JSON bodies. All endpoints except `/ping` require
`Authorization: Bearer <token>`.

### `GET /api/v1/ping` (no token)
→ `200 {"app":"rustloader","api":1,"version":"0.9.0"}`
Used for port discovery and "app is running" state. Reveals presence only.

### `POST /api/v1/download`
```jsonc
{
  "url": "https://…",              // required: the thing to download
  "page_url": "https://…",         // the tab URL (referer / cookie scope)
  "cookies": [                      // optional, from chrome.cookies.getAll
    {"name":"…","value":"…","domain":".example.com","path":"/",
     "secure":true,"httpOnly":false,"expires":1793577600}
  ],
  "headers": {"user_agent":"…","referer":"https://…"},  // optional
  "quality": "1080",               // optional: Best|Worst|<height>
  "output_format": "mp4"           // optional: keys from OutputFormat::as_key
}
```
→ `202 {"accepted":true}` — accepted means *handed to the GUI*, which shows a
non-intrusive banner/toast ("URL received from Chrome") and starts extraction
through the normal `ExtractInfo` flow. Errors: `401` bad/missing token,
`400` malformed, `422` non-http(s) URL (reuse `clipboard_monitor::detect_url`
validation), `429` if flooded.

Versioning: `api` field in ping + `/api/v1/` path prefix; unknown JSON fields
ignored (forward-compatible).

### App-side handling (invariants respected)
- Server task → `mpsc` → iced subscription → `Message::BridgeDownloadRequested{…}`
  → GUI sends `BackendCommand::ExtractInfo` / later `StartDownload` (**I-2**, **I-4**).
- Cookies land as a Netscape-format temp file under the app config dir
  (mode `0600`, one per request, deleted when the task finishes) →
  `CookieConfig { file }` (**I-7**: no hand-assembled `--cookies*` flags).
- Server impl: `hyper` 1.x server features (`http1` + `server` +
  `hyper-util`) — already in `Cargo.lock` via reqwest, so ~zero new audit
  surface. The bridge accept-loop is a plain tokio task; no new runtime.
- The bridge is **OFF by default**; a Settings → "Browser integration"
  section holds the toggle, the pairing token (shown as copyable code,
  regenerable), and the port. Stored via the existing settings KV rows.

## 6. Security model

Trust boundary: same OS user. The bridge defends against *other origins*, not
against the user's own account (the settings DB, like every browser's cookie
jar, is already user-readable).

| Threat | Defense |
|---|---|
| Malicious web page POSTing to localhost | 128-bit random token required; pages can't read extension storage; no CORS `Access-Control-Allow-Origin` echo — extension background fetch doesn't need page CORS |
| DNS rebinding (`Host: evil.com` resolving to 127.0.0.1) | Reject unless `Host` is `127.0.0.1:<port>`/`localhost:<port>` |
| Off-machine access | Bind `127.0.0.1` only, never `0.0.0.0`; no exceptions |
| Token brute force | 128-bit token; constant-time compare; 429 rate-limit on auth failures |
| Token leakage | Never logged (tracing filters); shown only in Settings on demand |
| Cookie file exposure | `0600`, app-config dir (not `/tmp`), per-task, deleted on completion; never logged |
| Malicious/oversized payloads | 1 MiB body cap; serde strict types; URL validated http(s)-only before touching yt-dlp |

`yt-dlp` invocation of extension-supplied URLs is unchanged from user-typed
URLs (same extraction pipeline, same timeout bounds — **I-1**).

## 7. Phased build plan

**Phase 1 — smallest end-to-end slice (Chrome, app running):**
1. App: `bridge` module — hyper loopback server, `/ping` + `/download`,
   token gen + settings storage, OFF-by-default toggle + token UI in
   Settings, `Message::BridgeDownloadRequested`, temp-cookie-file plumbing
   into `CookieConfig`, banner "URL received from browser".
2. Extension (new top-level `extension/chrome/` dir or sibling repo —
   recommend in-repo for now): manifest + service worker + context menu
   ("Download with Rustloader" → page/link URL + cookies) + bridge client +
   options/pairing page. Loaded unpacked (dev).
3. Acceptance: right-click a YouTube page → rustloader (already open) shows
   the banner, extracts, and the download appears in the queue with the
   page's cookies applied. CI green on 3 OSes (server code is
   platform-neutral; extension has no CI initially).

**Phase 2 — "IDM feel":** toolbar popup with quality/`OutputFormat` selection
(protocol already carries them), observational media sniffer + per-tab
detected-media list + action badge, `Download with Rustloader` on sniffed
media URLs.

**Phase 3 — multi-browser + store:** Firefox port (MV3 `browser.*`, different
cookie API nuances, native-messaging *not* needed — same localhost bridge),
Edge (Chromium, near-free), Chrome Web Store + AMO packaging/signing.

**Phase 4 (optional, separate decision) — app-not-running UX:**
`rustloader://` scheme registration (Windows registry, Linux `.desktop`,
macOS `CFBundleURLTypes` + a `kAEGetURL` Apple-event handler or winit ≥0.31)
**plus** a single-instance guard so a scheme launch forwards into the running
instance via the same loopback bridge. This phase is where single-instance
behaviour gets designed; it is deliberately not a Phase-1 dependency.

Out of scope for all phases: MITM/proxy capture, DRM-protected streams,
blocking webRequest.

## 8. Open questions for the maintainer

1. Extension in-repo (`extension/chrome/`) vs separate repo? (In-repo keeps
   the protocol and server in one PR-able unit; store review artifacts may
   later argue for a split.)
2. Pairing UX for Phase 1: manual token paste (least code) vs an
   omniget-style **single-use, ~120 s pairing window** (user clicks "Pair"
   in the app; the extension fetches the token from `GET /api/v1/pair`
   while the window is open — an atomic deadline + one endpoint).
   Recommendation: manual paste in Phase 1, pairing window in Phase 2 —
   promote it into Phase 1 only if Phase 1 lands under budget.
3. Port range 46150–46159 is arbitrary — any preference/known conflicts?
