# Rustloader App UI Kit

Full-fidelity recreation of the Rustloader desktop app at its fixed 1080×720, non-resizable window — the target design (dark-first, rust/copper) grounded in the real feature set of `reference/rustloader-repo/src/gui/`.

- `index.html` — interactive demo: letterboxed fixed canvas, live simulated queue (segments fill, speeds tick), pause/resume/cancel/retry work, dark/light toggle, all three views.
- `AppShell.jsx` — window chrome: icon rail nav, header (identity + global StatTiles), view switching, simulation state.
- `DownloadsScreen.jsx` — hero UrlBar, clipboard banner, active queue with every download-item state.
- `HistoryScreen.jsx` — completed rows + empty state.
- `SettingsScreen.jsx` — location, performance sliders, quality, cookies-from-browser, clipboard-monitoring opt-in.

Web-only effects (glass blur, gradient hairline, glow) degrade to solids in Iced — see readme.md.
