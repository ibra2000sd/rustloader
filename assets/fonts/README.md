# Bundled fonts

Geist and Geist Mono, the design-system typefaces (`design-system/tokens/typography.css`):
UI text uses **Geist**, all numerals/data (speeds, sizes, ETAs, %, paths, timestamps)
use **Geist Mono**.

- Source: https://github.com/vercel/geist-font, release **v1.7.2**
  (`geist-font-v1.7.2.zip`, static TTFs).
- License: SIL Open Font License 1.1 — see [OFL.txt](OFL.txt) (copied from the
  upstream repository). OFL permits bundling and redistribution with the app.
- Weights bundled match the design-system spec (`tokens/fonts.css`):
  Geist 400/500/600/700, Geist Mono 400/500/600.

The files are embedded into the binary via `include_bytes!` in
`src/gui/theme.rs` (`font_bytes()`) and registered with Iced through
`Settings::fonts` in `src/main.rs`.
