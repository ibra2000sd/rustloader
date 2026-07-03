---
name: rustloader-design
description: Use this skill to generate well-branded interfaces and assets for Rustloader, either for production or throwaway prototypes/mocks/etc. Contains essential design guidelines, colors, type, fonts, assets, and UI kit components for prototyping.
user-invocable: true
---

Read the README.md (readme.md) file within this skill, and explore the other available files.
If creating visual artifacts (slides, mocks, throwaway prototypes, etc), copy assets out and create static HTML files for the user to view. If working on production code, you can copy assets and read the rules here to become an expert in designing with this brand.
If the user invokes this skill without any other guidance, ask them what they want to build or design, ask some questions, and act as an expert designer who outputs HTML artifacts _or_ production code, depending on the need.

Key facts:
- Dark-first, fixed 1080×720 non-resizable desktop window; rust/copper accent (`--rust-500 #CF6F38`), amber reserved for live data.
- All numerals/data in Geist Mono; UI in Geist. Tokens in `tokens/`, entry `styles.css`.
- Signature visual: the multi-segment parallel progress bar (`components/data/SegmentBar.jsx`).
- Full app recreation: `ui_kits/app/index.html`.
- Iced (Rust) cannot render blur/glow/gradient-hairline effects — those are web-only; degrade to solid surfaces.
