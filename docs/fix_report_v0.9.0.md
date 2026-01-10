# Rustloader v0.9.0 – Critical Fixes and UI Improvements

## Bugs Fixed

- FIXED: Removed invalid `-f all` from yt-dlp extraction to make extraction format-agnostic.
- FIXED: Removed duplicate quality dropdown from main view to avoid conflicting format selection UI.
- FIXED: Integrated format selector modal to appear after extraction and drive format choice.
- IMPROVED: Error messages now provide clearer, user-friendly guidance, including format availability issues.
- IMPROVED: Added loading indicator during extraction ("Analyzing video...") for better feedback.

## Files Modified

- Extractor
  - [src/extractor/ytdlp.rs#L48](src/extractor/ytdlp.rs#L48): Removed `-f all` from `extract_info_impl()`.

- GUI – Main View
  - [src/gui/views/main_view.rs#L92](src/gui/views/main_view.rs#L92): Added loading state during extraction.
  - Removed the always-visible quality dropdown section (previously in options block).

- GUI – App Integration
  - [src/gui/app.rs#L325](src/gui/app.rs#L325): Show `FormatSelector` after successful extraction.
  - [src/gui/app.rs#L729](src/gui/app.rs#L729): Render selector modal overlay via `selector.view().map(Message::FormatSelector)`.
  - [src/gui/app.rs#L517](src/gui/app.rs#L517): Use `selector.selected_format_id.clone()` when confirming selection.
  - [src/gui/app.rs#L890](src/gui/app.rs#L890): Improved `make_error_user_friendly()` to handle format availability errors.

## Lines Changed (Key Locations)

- Extraction: [src/extractor/ytdlp.rs#L48](src/extractor/ytdlp.rs#L48)
- GUI Loading: [src/gui/views/main_view.rs#L92](src/gui/views/main_view.rs#L92)
- Show Modal: [src/gui/app.rs#L325](src/gui/app.rs#L325), [src/gui/app.rs#L729](src/gui/app.rs#L729)
- Confirm Selection: [src/gui/app.rs#L517](src/gui/app.rs#L517)
- Error Message: [src/gui/app.rs#L890](src/gui/app.rs#L890)

## Test Results Summary

- Build: `cargo build --release` – succeeded.
- Tests: `cargo test --all` – 99 unit tests passed; all integration and stress tests passed.
- Manual: Ready to validate with real URLs; GUI now shows modal after extraction and respects selected format.

## Notes

- Backend actor and message flow unchanged.
- Download engine untouched.
- Future improvement: consider refining `VideoQuality` display labels if reintroducing a summary dropdown in Settings.
