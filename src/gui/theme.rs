//! Rustloader design-system theme — dark, rust/copper accent.
//!
//! Every color, radius, and font here maps 1:1 to a token in
//! `design-system/tokens/*.css` (the CSS custom-property name is noted next to
//! each constant). Web-only effects (backdrop blur, gradient hairlines,
//! custom easing) degrade to the solid fallbacks documented in
//! `design-system/readme.md`: glass panels become solid `--bg-2` cards, the
//! gradient hairline becomes a 1px `rgba(244,241,236,0.05)` line, and motion
//! stays on Iced defaults.

use iced::widget::{button, container, scrollable, text_input};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector};
use std::borrow::Cow;

/// Build a [`Color`] from 8-bit channels in a `const` context
/// (`Color::from_rgb8` is not `const` in iced 0.12).
const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// Like [`rgb8`] with an alpha channel.
const fn rgba8(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
}

// --- Surfaces: OLED-friendly warm near-blacks (tokens/colors.css) ---

pub const BG_0: Color = rgb8(0x0A, 0x09, 0x08); // --bg-0, window base
pub const BG_1: Color = rgb8(0x12, 0x11, 0x10); // --bg-1, raised panel
pub const BG_2: Color = rgb8(0x1A, 0x18, 0x15); // --bg-2, card (glass fallback)
pub const BG_3: Color = rgb8(0x23, 0x20, 0x1B); // --bg-3, hover / nested
pub const BG_INPUT: Color = rgb8(0x14, 0x12, 0x0F); // --bg-input
pub const BG_ACCENT_SOFT: Color = rgba8(0xCF, 0x6F, 0x38, 0.12); // --bg-accent-soft
pub const BG_DANGER_SOFT: Color = rgba8(0xD9, 0x50, 0x3B, 0.12); // --bg-danger-soft

// --- Text: warm whites ---

pub const FG_1: Color = rgb8(0xF4, 0xF1, 0xEC); // --fg-1, primary
pub const FG_2: Color = rgb8(0xB7, 0xB0, 0xA6); // --fg-2, secondary
pub const FG_3: Color = rgb8(0x7E, 0x78, 0x6E); // --fg-3, tertiary / labels
pub const FG_DISABLED: Color = rgb8(0x55, 0x50, 0x4A); // --fg-disabled
pub const FG_ON_ACCENT: Color = rgb8(0x15, 0x0C, 0x06); // --fg-on-accent

// --- Accent: rust/copper ramp ---

pub const RUST_400: Color = rgb8(0xDE, 0x8B, 0x54); // --rust-400, hover
pub const RUST_500: Color = rgb8(0xCF, 0x6F, 0x38); // --rust-500, primary accent
pub const RUST_600: Color = rgb8(0xA9, 0x57, 0x2B); // --rust-600, pressed

/// Amber is reserved for live data (speeds, hot numerals) — never for chrome.
pub const AMBER_400: Color = rgb8(0xE9, 0xB4, 0x4C); // --amber-400 / --data-hot

// --- Semantic status (warm-tuned) ---

pub const SUCCESS: Color = rgb8(0x3A, 0xAE, 0x6F); // --success-500
pub const WARNING: Color = rgb8(0xD9, 0x9A, 0x2B); // --warning-500
pub const DANGER: Color = rgb8(0xD9, 0x50, 0x3B); // --danger-500
pub const DANGER_400: Color = rgb8(0xEA, 0x6E, 0x5A); // --danger-400

// --- Borders & hairlines ---

pub const BORDER_HAIRLINE: Color = rgba8(0xF4, 0xF1, 0xEC, 0.08); // --border-hairline
pub const BORDER_STRONG: Color = rgba8(0xF4, 0xF1, 0xEC, 0.14); // --border-strong
pub const BORDER_ACCENT: Color = rgba8(0xCF, 0x6F, 0x38, 0.45); // --border-accent
pub const PROGRESS_TRACK: Color = rgba8(0xF4, 0xF1, 0xEC, 0.07); // --progress-track

// --- Compatibility aliases used across the existing widget code ---

pub const TEXT_PRIMARY: Color = FG_1;
pub const TEXT_SECONDARY: Color = FG_2;
pub const ACCENT: Color = RUST_500;

// --- Radius scale (tokens/spacing.css): precise, not bubbly ---

pub const RADIUS_BAR: f32 = 4.0; // --radius-xs, progress bars / tags
pub const RADIUS_CONTROL: f32 = 8.0; // --radius-md, buttons / inputs
pub const RADIUS_CARD: f32 = 10.0; // --radius-lg, cards

// --- Fonts (tokens/typography.css): Geist for UI, Geist Mono for data ---

pub const FONT_UI: Font = Font::with_name("Geist");
pub const FONT_UI_SEMIBOLD: Font = Font {
    weight: iced::font::Weight::Semibold,
    ..Font::with_name("Geist")
};
/// Every speed, size, ETA, %, path, and timestamp renders in Geist Mono.
pub const FONT_MONO: Font = Font::with_name("Geist Mono");

/// Shaping for content-derived text: video titles, filenames/paths, error
/// messages, and URLs — anything whose characters we don't control.
///
/// Geist and Geist Mono are Latin-only. Iced's default `Shaping::Basic` maps
/// every character through the requested font's charmap alone, so Arabic,
/// CJK, and emoji render as tofu (□). `Shaping::Advanced` enables
/// cosmic-text's per-script fallback to the system fonts already loaded in
/// the font database, keeping Geist first for the glyphs it has. Latin-only
/// UI chrome (buttons, labels) stays on the cheaper `Basic` default.
pub const SHAPING_CONTENT: iced::widget::text::Shaping = iced::widget::text::Shaping::Advanced;

/// The bundled Geist / Geist Mono binaries (OFL-1.1, `assets/fonts/OFL.txt`),
/// registered with Iced via `Settings::fonts`. Weights follow
/// `tokens/fonts.css`: Geist 400/500/600/700, Geist Mono 400/500/600.
pub fn font_bytes() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../../assets/fonts/Geist-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Geist-Medium.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Geist-SemiBold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/Geist-Bold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/GeistMono-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/GeistMono-Medium.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/GeistMono-SemiBold.ttf").as_slice()),
    ]
}

/// The application theme: Iced's palette mapped to the design tokens, so
/// built-in widgets (pick_list, slider, toggler, tooltip) derive dark styles
/// consistent with the custom ones below.
pub fn app_theme() -> Theme {
    Theme::custom(
        "Rustloader Dark".to_string(),
        iced::theme::Palette {
            background: BG_0,
            text: FG_1,
            primary: RUST_500,
            success: SUCCESS,
            danger: DANGER,
        },
    )
}

// --- Shadows (tokens/effects.css): soft, warm-black ---

/// `--shadow-1`: 0 1px 2px rgba(0,0,0,0.3)
const SHADOW_1: Shadow = Shadow {
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
    offset: Vector::new(0.0, 1.0),
    blur_radius: 2.0,
};

/// `--shadow-accent`: 0 4px 16px rgba(207,111,56,0.25) — primary CTA only.
const SHADOW_ACCENT: Shadow = Shadow {
    color: rgba8(0xCF, 0x6F, 0x38, 0.25),
    offset: Vector::new(0.0, 4.0),
    blur_radius: 16.0,
};

const NO_SHADOW: Shadow = Shadow {
    color: Color::TRANSPARENT,
    offset: Vector::new(0.0, 0.0),
    blur_radius: 0.0,
};

// --- Container Styles ---

/// The window background: flat `--bg-0` near-black. The design system bans
/// image backgrounds and big gradients; depth comes from elevation steps.
pub struct WindowContainer;

impl container::StyleSheet for WindowContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(FG_1),
            background: Some(Background::Color(BG_0)),
            ..Default::default()
        }
    }
}

/// Card surface: solid `--bg-2` (the documented Iced fallback for the
/// web-only glass), 1px hairline, `--radius-lg`, `--shadow-1`.
pub struct GlassContainer;

impl container::StyleSheet for GlassContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(FG_1),
            background: Some(Background::Color(BG_2)),
            border: Border {
                color: BORDER_HAIRLINE,
                width: 1.0,
                radius: RADIUS_CARD.into(),
            },
            shadow: SHADOW_1,
        }
    }
}

/// Sidebar: raised `--bg-1` panel with a hairline edge.
pub struct SidebarContainer;

impl container::StyleSheet for SidebarContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(FG_2),
            background: Some(Background::Color(BG_1)),
            border: Border {
                color: BORDER_HAIRLINE,
                width: 1.0,
                radius: 0.0.into(),
            },
            shadow: NO_SHADOW,
        }
    }
}

// --- Button Styles ---

/// Primary CTA: solid rust fill; hover brightens to `--accent-hover` with the
/// accent glow, press darkens to `--accent-pressed` (no scale-shrink).
pub struct PrimaryButton;

impl button::StyleSheet for PrimaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(RUST_500)),
            text_color: FG_ON_ACCENT,
            border: Border {
                radius: RADIUS_CONTROL.into(),
                ..Default::default()
            },
            shadow: NO_SHADOW,
            shadow_offset: Vector::new(0.0, 0.0),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(RUST_400)),
            shadow: SHADOW_ACCENT,
            ..self.active(style)
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(RUST_600)),
            ..self.active(style)
        }
    }
}

/// Secondary: card surface with a strong border; hover steps up one
/// elevation (`--surface-hover`).
pub struct SecondaryButton;

impl button::StyleSheet for SecondaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_2)),
            text_color: FG_1,
            border: Border {
                radius: RADIUS_CONTROL.into(),
                color: BORDER_STRONG,
                width: 1.0,
            },
            shadow: NO_SHADOW,
            shadow_offset: Vector::new(0.0, 0.0),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_3)),
            ..self.active(style)
        }
    }
}

/// Sidebar navigation entries: active gets the soft accent wash.
pub enum SidebarButtonStyle {
    Active,
    Inactive,
}

impl button::StyleSheet for SidebarButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        match self {
            Self::Active => button::Appearance {
                background: Some(Background::Color(BG_ACCENT_SOFT)),
                text_color: FG_1,
                border: Border {
                    radius: RADIUS_CONTROL.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Self::Inactive => button::Appearance {
                background: None,
                text_color: FG_2,
                border: Border {
                    radius: RADIUS_CONTROL.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        match self {
            Self::Active => self.active(style),
            Self::Inactive => button::Appearance {
                text_color: FG_1,
                background: Some(Background::Color(BG_3)),
                border: Border {
                    radius: RADIUS_CONTROL.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

/// Ghost button: transparent at rest; hover steps up one elevation and
/// brightens the text one step.
pub struct IconButton;

impl button::StyleSheet for IconButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: FG_2,
            border: Border {
                radius: RADIUS_CONTROL.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            text_color: FG_1,
            background: Some(Background::Color(BG_3)),
            border: Border {
                radius: RADIUS_CONTROL.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// Destructive ghost: danger text, soft danger wash on hover.
pub struct DestructiveButton;

impl button::StyleSheet for DestructiveButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: DANGER_400,
            border: Border {
                radius: RADIUS_CONTROL.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_DANGER_SOFT)),
            text_color: DANGER_400,
            border: Border {
                radius: RADIUS_CONTROL.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

// --- Input Styles ---

pub struct InputStyle;

impl text_input::StyleSheet for InputStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(BG_INPUT),
            border: Border {
                radius: RADIUS_CONTROL.into(),
                width: 1.0,
                color: BORDER_STRONG,
            },
            icon_color: FG_3,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            border: Border {
                color: BORDER_ACCENT,
                ..active.border
            },
            ..active
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        FG_3
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        FG_1
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        rgba8(0xCF, 0x6F, 0x38, 0.35)
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            background: Background::Color(BG_1),
            ..active
        }
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        FG_DISABLED
    }
}

pub struct InputErrorStyle;

impl text_input::StyleSheet for InputErrorStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(BG_INPUT),
            border: Border {
                radius: RADIUS_CONTROL.into(),
                width: 1.0,
                color: DANGER_400,
            },
            icon_color: DANGER_400,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        FG_3
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        FG_1
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        rgba8(0xD9, 0x50, 0x3B, 0.3)
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            background: Background::Color(BG_1),
            ..active
        }
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        FG_DISABLED
    }
}

// --- Scrollable Styles ---

pub struct ScrollableStyle;

impl scrollable::StyleSheet for ScrollableStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> scrollable::Appearance {
        scrollable::Appearance {
            container: container::Appearance::default(),
            scrollbar: scrollable::Scrollbar {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border::default(),
                scroller: scrollable::Scroller {
                    color: BORDER_STRONG,
                    border: Border {
                        radius: RADIUS_BAR.into(),
                        ..Default::default()
                    },
                },
            },
            gap: None,
        }
    }

    fn hovered(
        &self,
        style: &Self::Style,
        is_mouse_over_scrollbar: bool,
    ) -> scrollable::Appearance {
        let active = self.active(style);
        if is_mouse_over_scrollbar {
            scrollable::Appearance {
                scrollbar: scrollable::Scrollbar {
                    scroller: scrollable::Scroller {
                        color: rgba8(0xF4, 0xF1, 0xEC, 0.25),
                        ..active.scrollbar.scroller
                    },
                    ..active.scrollbar
                },
                ..active
            }
        } else {
            active
        }
    }
}

// --- Progress Bar Styles ---
//
// Bar states per the design system: active = rust, completed = green,
// paused = dimmed amber, stalled = amber (the web pulse animation degrades
// to a solid fill in Iced). All bars sit on `--progress-track` with
// `--radius-xs` corners.

pub struct ProgressBarStyle;

impl iced::widget::progress_bar::StyleSheet for ProgressBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(PROGRESS_TRACK),
            bar: Background::Color(RUST_500),
            border_radius: RADIUS_BAR.into(),
        }
    }
}

pub struct ProgressBarCompleted;

impl iced::widget::progress_bar::StyleSheet for ProgressBarCompleted {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(PROGRESS_TRACK),
            bar: Background::Color(SUCCESS),
            border_radius: RADIUS_BAR.into(),
        }
    }
}

pub struct ProgressBarDimmed;

impl iced::widget::progress_bar::StyleSheet for ProgressBarDimmed {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(PROGRESS_TRACK),
            bar: Background::Color(rgba8(0xE9, 0xB4, 0x4C, 0.5)),
            border_radius: RADIUS_BAR.into(),
        }
    }
}

/// v0.6.0: Style for stalled downloads (amber/warning)
pub struct ProgressBarStalled;

impl iced::widget::progress_bar::StyleSheet for ProgressBarStalled {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(PROGRESS_TRACK),
            bar: Background::Color(AMBER_400),
            border_radius: RADIUS_BAR.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_match_the_token_spec() {
        // tokens/fonts.css: Geist 400/500/600/700 + Geist Mono 400/500/600.
        assert_eq!(font_bytes().len(), 7);
    }

    #[test]
    fn bundled_fonts_are_valid_truetype() {
        for (i, bytes) in font_bytes().iter().enumerate() {
            // TrueType sfnt version 1.0: 00 01 00 00.
            assert_eq!(
                &bytes[0..4],
                &[0x00, 0x01, 0x00, 0x00],
                "font #{i} is not a TrueType binary"
            );
        }
    }

    #[test]
    fn palette_matches_design_tokens() {
        // Spot-check the load-bearing token values against tokens/colors.css.
        let as_rgb8 = |c: Color| {
            (
                (c.r * 255.0).round() as u8,
                (c.g * 255.0).round() as u8,
                (c.b * 255.0).round() as u8,
            )
        };
        assert_eq!(as_rgb8(BG_0), (0x0A, 0x09, 0x08)); // --bg-0
        assert_eq!(as_rgb8(FG_1), (0xF4, 0xF1, 0xEC)); // --fg-1
        assert_eq!(as_rgb8(ACCENT), (0xCF, 0x6F, 0x38)); // --rust-500
        assert_eq!(as_rgb8(AMBER_400), (0xE9, 0xB4, 0x4C)); // --amber-400
        assert_eq!(as_rgb8(SUCCESS), (0x3A, 0xAE, 0x6F)); // --success-500
        assert_eq!(as_rgb8(WARNING), (0xD9, 0x9A, 0x2B)); // --warning-500
        assert_eq!(as_rgb8(DANGER), (0xD9, 0x50, 0x3B)); // --danger-500
    }
}
