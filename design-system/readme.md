# Rustloader Design System

**Rustloader** is a fast, lightweight, cross-platform download manager written in Rust. It downloads video/audio from 1800+ sites (via yt-dlp) and any direct file with a signature **multi-segment parallel engine** (up to 16 concurrent byte-range segments, merged on completion), with byte-level resume, download history, opt-in clipboard monitoring, and quality/format selection (480/720/1080 · mp4/mp3). The current shipping GUI is built in **Iced (Rust)**; this design system defines the target 2026 visual identity — dark-first, precision-instrument, rust/copper accent — for the app's evolution (aspirational for Iced, buildable directly on a web shell like Tauri).

Brand personality: **fast · lightweight · precise · native · trustworthy · no-bloat.** The foil is heavy Java/Electron bloatware. Rustloader looks like a precision instrument: engineered, confident, calm.

## Sources
- GitHub: https://github.com/ibra2000sd/rustloader — real feature set, copy, GUI structure (`src/gui/`), and app icon. A snapshot of the GUI source lives in `reference/rustloader-repo/`. Explore the repo further to ground new designs in the real product.
- Design brief (attached by the user): dark-first, fixed 1080×720 non-resizable window, rust/copper/amber accent, mono-inflected numerals, bento cards, subtle glass, functional motion.

## Hard constraints
- **Fixed window: 1080×720, non-resizable.** No responsive breakpoints. Every screen is designed to this exact canvas.
- **Dark theme is the default**; light theme adapts via `[data-theme="light"]`.
- Keyboard-navigable focus states (`--focus-ring`), reduced-motion honoured (`tokens/motion.css` zeroes durations).
- Iced cannot render backdrop-blur, gradient hairlines, or custom easing — those are **web-only** (flagged in token comments); fall back to solid `--surface-card` and default motion in Iced.

## CONTENT FUNDAMENTALS
Copy is plain, direct, and honest — engineering-grade prose, not marketing.
- **Sentence case everywhere** ("Detect copied URLs", "No download history yet"). Title Case only for view titles and proper features ("Clipboard Monitoring", "Download Location").
- **Verb-first button labels**: "Download", "Pause", "Resume", "Retry", "Show in Folder", "Clear Completed", "Save Settings". Never "OK"/"Submit".
- **Second person, active, reassuring about safety**: "When you copy a web link, you'll be asked whether to download it — nothing downloads automatically, and clipboard contents are never stored or sent anywhere." Privacy-sensitive features explain what they never do.
- **Explicit over cute**: "Unknown size" rather than a misleading "0.0 MB"; errors are classified and paired with a recovery hint ("Check your connection and retry").
- **Progressive status verbs** with ellipsis for transitions: "Extracting…", "Pausing…", "Cancelling…", "Retrying…".
- **Data is terse and unit-suffixed**: `4.2 MB/s`, `128.5 MB / 704.2 MB`, `ETA 0:42`, `16 segments`. Always mono.
- **No emoji in the target UI.** (The repo's README uses emoji; the app UI uses a few unicode glyphs like ⚠/✕ — the design system replaces these with the Lucide icon set.)
- Empty states are two lines: a calm statement + what will happen ("No active downloads / Your downloads will appear here").

## VISUAL FOUNDATIONS
- **Color**: OLED-friendly warm near-blacks (`#0A0908 → #23201B` in four elevation steps). One accent family: rust/copper (`--rust-500 #CF6F38`), with **amber (`--amber-400 #E9B44C`) reserved for live data** (speeds, hot numerals). Status: warm-tuned green/amber/red. Light theme is warm paper (`#F6F3EE`), same accent shifted darker for contrast.
- **Type**: Geist (UI) + Geist Mono (all numerals/data — speeds, sizes, ETAs, %, paths, timestamps). Uppercase mono micro-labels with `0.08em` tracking for section/tags. Scale 10–28px; hero numeral 28px semibold mono.
- **Spacing**: 4px base scale, dense-but-calm; cards pad 16px, view gutters 24–32px.
- **Radius**: precise, not bubbly — 4px bars/tags, 8px buttons/inputs, 10px cards, 14px panels.
- **Backgrounds**: flat near-black window; **no image backgrounds, no big gradients**. Depth comes from elevation steps + 1px hairlines (`rgba(white, 0.08)`) + a 1px inset top-highlight that reads "machined". Optional gradient hairline border (`--border-gradient`) on hero panels — web-only.
- **Glass**: subtle only — translucent panel (`--bg-glass`) + `blur(14px)` for overlays/banners; never heavy blur, never on list rows. Web-only; Iced falls back to solid.
- **Shadows**: soft warm-black ambient (`--shadow-1..3`); accent glow (`--shadow-accent`) only under the primary CTA and active segment bars.
- **Motion**: functional only. 120ms hover, 220ms state change, 400ms view transitions; `--ease-out`. Active transfers pulse a dot (`rl-pulse`, 2s); segment bars fill left-to-right. Reduced motion zeroes durations and stops pulses. No bounces, no decorative loops.
- **Hover**: background steps up one elevation (`--surface-hover`); text brightens one step. **Press**: accent darkens to `--accent-pressed`, shadow tightens; no scale-shrink.
- **Focus**: 2px offset ring in rust (`--focus-ring`) — always visible, keyboard-first.
- **Cards**: `--surface-card` + 1px hairline + `--radius-lg` + `--shadow-1`; bento layout for stats (asymmetric, dense-but-calm). No colored left-border accents.
- **Progress**: the signature visual is the **segmented progress bar** — N thin cells (one per parallel segment) filling independently in rust, merging visually into one bar; completed = green, paused = dimmed amber, stalled = amber pulse, error = red.
- **Imagery**: none in-app; the only artwork is the app icon. No illustrations.

## ICONOGRAPHY
- The repo ships **no UI icon set** — the Iced GUI uses text labels plus occasional unicode glyphs (⚠, ✕, ←, 💡). The only real brand asset is the **app icon** (rocket + gear in an orange ring): `assets/icons/icon_256x256.png` (also 128/32). **There is no wordmark/logo** — render "rustloader" in Geist Semibold, lowercase, with the app icon beside it.
- Target UI icons: **Lucide** (stroke 1.5–2px, matches the precision aesthetic), used via the `Icon` component (`components/core/Icon.jsx`), which embeds copied Lucide path data (ISC license) — a **flagged substitution**, since no source icon set exists. In plain HTML you may instead load Lucide from CDN.
- No emoji as icons. Status is conveyed by color + icon (pause, play, check, alert-triangle, x).

## Tokens & fonts
- Entry: `styles.css` → `tokens/{fonts,colors,typography,spacing,effects,motion}.css`.
- Fonts load from Google Fonts (Geist, Geist Mono) — **substitution**: repo has no font binaries; replace `tokens/fonts.css` with licensed `@font-face` files if the brand adopts specific faces.

## Index
- `readme.md` — this guide
- `styles.css`, `tokens/` — design tokens (dark default + `[data-theme="light"]`)
- `assets/icons/` — app icon (16–256px)
- `guidelines/` — foundation specimen cards (Design System tab)
- `components/core/` — Icon, Button, IconButton, Card, StatusBadge
- `components/forms/` — Input, Select, Slider, Toggle
- `components/data/` — ProgressBar, SegmentBar, StatTile
- `components/downloads/` — DownloadItem, HistoryItem, ClipboardBanner, UrlBar
- `ui_kits/app/` — full 1080×720 interactive recreation: Downloads (main), History, Settings + all download-item states, dark/light toggle
- `SKILL.md` — agent skill entry point

### Intentional additions
- `Icon` — glyph wrapper for the Lucide set (no icon system existed in source).
- `SegmentBar` — the brief's signature multi-segment progress visual (the Iced app renders a single bar today).
- `StatTile` — header global-stats tile required by the brief.
