#![allow(clippy::approx_constant, dead_code)]

//! Custom theme definitions for the application - Light Theme

use iced::widget::{button, container, progress_bar, scrollable, text_input};
use iced::{Background, Border, Color, Gradient, Shadow, Theme, Vector};

// --- Rustloader Design System v1.0 ---

pub struct RustloaderTheme;

impl RustloaderTheme {
    // Colors
    pub const PRIMARY: Color = Color::from_rgb(0.388, 0.400, 0.945); // #6366F1 (Indigo 500)
    pub const SECONDARY: Color = Color::from_rgb(0.545, 0.361, 0.965); // #8B5CF6 (Violet 500)
    pub const SUCCESS: Color = Color::from_rgb(0.063, 0.725, 0.506); // #10B981 (Emerald 500)
    pub const WARNING: Color = Color::from_rgb(0.961, 0.620, 0.094); // #F59E0B (Amber 500)
    pub const ERROR: Color = Color::from_rgb(0.937, 0.267, 0.267); // #EF4444 (Red 500)

    pub const BG_LIGHT: Color = Color::from_rgb(0.973, 0.980, 0.988); // #F8FAFC (Slate 50)
    pub const BG_DARK: Color = Color::from_rgb(0.118, 0.161, 0.231); // #1E293B (Slate 800)

    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.122, 0.161, 0.216); // #1E293B (Slate 800)
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.392, 0.455, 0.545); // #64748B (Slate 500)
    pub const TEXT_DISABLED: Color = Color::from_rgb(0.616, 0.639, 0.667); // Slate 400

    // Spacing
    pub const SPACING_XS: u16 = 4;
    pub const SPACING_SM: u16 = 8;
    pub const SPACING_MD: u16 = 16;
    pub const SPACING_LG: u16 = 24;
    pub const SPACING_XL: u16 = 32;

    // Border radius
    pub const RADIUS_CARD: f32 = 12.0;
    pub const RADIUS_BUTTON: f32 = 8.0;
    pub const RADIUS_INPUT: f32 = 6.0;
}

// Backward compatibility constants
pub const INDIGO_500: Color = RustloaderTheme::PRIMARY;
pub const GRAY_800: Color = RustloaderTheme::TEXT_PRIMARY;
pub const GRAY_700: Color = RustloaderTheme::TEXT_SECONDARY;
pub const GRAY_200: Color = Color::from_rgb(0.898, 0.906, 0.922);
pub const WHITE: Color = Color::WHITE;
pub const WARNING: Color = RustloaderTheme::WARNING;
pub const DANGER: Color = RustloaderTheme::ERROR;
pub const SUCCESS: Color = RustloaderTheme::SUCCESS; // Added
pub const ACCENT: Color = RustloaderTheme::SECONDARY; // Added
pub const TEXT_PRIMARY: Color = RustloaderTheme::TEXT_PRIMARY;
pub const TEXT_SECONDARY: Color = RustloaderTheme::TEXT_SECONDARY;
pub const GRAY_500: Color = RustloaderTheme::TEXT_DISABLED;
pub const GRAY_400: Color = RustloaderTheme::TEXT_DISABLED; // approx
pub const GRAY_100: Color = RustloaderTheme::BG_LIGHT;

// Missing containers
pub struct GlassContainer;
impl container::StyleSheet for GlassContainer {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.9))),
            border: Border {
                color: GRAY_200,
                width: 1.0,
                radius: 12.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
            ..Default::default()
        }
    }
}

pub struct ScrollableStyle;
impl scrollable::StyleSheet for ScrollableStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> scrollable::Appearance {
        scrollable::Appearance {
            scrollbar: scrollable::Scrollbar {
                background: None,
                border: Border::default(),
                scroller: scrollable::Scroller {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                    border: Border::default(),
                },
            },
            gap: None,
            container: container::Appearance::default(),
        }
    }
    fn hovered(
        &self,
        style: &Self::Style,
        is_mouse_over_scrollbar: bool,
    ) -> scrollable::Appearance {
        self.active(style)
    }
}

pub struct IconButton;
impl button::StyleSheet for IconButton {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: RustloaderTheme::TEXT_SECONDARY,
            ..Default::default()
        }
    }
    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(GRAY_200)),
            text_color: RustloaderTheme::TEXT_PRIMARY,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

// ActionButton is defined later in file

pub struct MainGradientContainer;

impl container::StyleSheet for MainGradientContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(RustloaderTheme::TEXT_PRIMARY),
            background: Some(Background::Gradient(Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(2.356)) // 135 degrees
                    .add_stop(0.0, Color::from_rgb(0.941, 0.976, 1.0)) // Sky 50
                    .add_stop(0.5, Color::from_rgb(0.878, 0.906, 1.0)) // Indigo 50
                    .add_stop(1.0, Color::from_rgb(0.953, 0.910, 1.0)), // Purple 50
            ))),
            ..Default::default()
        }
    }
}

pub struct SidebarContainer;

impl container::StyleSheet for SidebarContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(RustloaderTheme::TEXT_SECONDARY),
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.7))),
            border: Border {
                color: GRAY_200,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }
}

pub struct DownloadCardStyle;

impl container::StyleSheet for DownloadCardStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::WHITE)),
            border: Border {
                color: GRAY_200,
                width: 1.0,
                radius: RustloaderTheme::RADIUS_CARD.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        }
    }
}

// --- Button Styles ---

pub struct PrimaryButton;

impl button::StyleSheet for PrimaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Gradient(Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(0.0))
                    .add_stop(0.0, RustloaderTheme::PRIMARY)
                    .add_stop(1.0, RustloaderTheme::SECONDARY),
            ))),
            text_color: Color::WHITE,
            border: Border {
                radius: RustloaderTheme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.388, 0.400, 0.945, 0.3),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            shadow_offset: Vector::new(0.0, 0.0),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            shadow: Shadow {
                color: Color::from_rgba(0.388, 0.400, 0.945, 0.4),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 20.0,
            },
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            shadow: Shadow {
                offset: Vector::new(0.0, 2.0),
                ..active.shadow
            },
            ..active
        }
    }
}

pub struct SecondaryButton;

impl button::StyleSheet for SecondaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::WHITE)),
            text_color: RustloaderTheme::TEXT_PRIMARY,
            border: Border {
                color: GRAY_200,
                width: 1.0,
                radius: RustloaderTheme::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Background::Color(GRAY_100)),
            ..active
        }
    }
}

pub enum SidebarButtonStyle {
    Active,
    Inactive,
}

impl button::StyleSheet for SidebarButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        match self {
            Self::Active => button::Appearance {
                background: Some(Background::Color(Color::from_rgba(
                    0.388, 0.400, 0.945, 0.1,
                ))),
                text_color: RustloaderTheme::PRIMARY,
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Self::Inactive => button::Appearance {
                background: None,
                text_color: RustloaderTheme::TEXT_SECONDARY,
                border: Border {
                    radius: 8.0.into(),
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
                background: Some(Background::Color(RustloaderTheme::BG_LIGHT)),
                text_color: RustloaderTheme::TEXT_PRIMARY,
                ..self.active(style)
            },
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}

pub struct ActionButton; // Small icon buttons (Pause, Cancel, etc.)

impl button::StyleSheet for ActionButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: RustloaderTheme::TEXT_SECONDARY,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(GRAY_200)),
            text_color: RustloaderTheme::TEXT_PRIMARY,
            ..self.active(_style)
        }
    }
}

pub struct DestructiveButton;

impl button::StyleSheet for DestructiveButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: RustloaderTheme::ERROR,
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.996, 0.886, 0.886))), // Red 100
            ..self.active(_style)
        }
    }
}

// --- Progress Bar ---
pub struct ProgressBarCompleted;
impl progress_bar::StyleSheet for ProgressBarCompleted {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> progress_bar::Appearance {
        progress_bar::Appearance {
            background: Background::Color(GRAY_200),
            bar: Background::Color(RustloaderTheme::SUCCESS),
            border_radius: 4.0.into(),
        }
    }
}

pub struct ProgressBarStalled;
impl progress_bar::StyleSheet for ProgressBarStalled {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> progress_bar::Appearance {
        progress_bar::Appearance {
            background: Background::Color(GRAY_200),
            bar: Background::Color(RustloaderTheme::WARNING),
            border_radius: 4.0.into(),
        }
    }
}

pub struct ProgressBarDimmed;
impl progress_bar::StyleSheet for ProgressBarDimmed {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> progress_bar::Appearance {
        progress_bar::Appearance {
            background: Background::Color(GRAY_200),
            bar: Background::Color(GRAY_400),
            border_radius: 4.0.into(),
        }
    }
}

pub struct ProgressBarStyle;

impl progress_bar::StyleSheet for ProgressBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> progress_bar::Appearance {
        progress_bar::Appearance {
            background: Background::Color(Color::from_rgb(0.878, 0.906, 1.0)), // Indigo 100
            bar: Background::Gradient(Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(0.0))
                    .add_stop(0.0, RustloaderTheme::PRIMARY)
                    .add_stop(1.0, RustloaderTheme::SECONDARY),
            )),
            border_radius: 4.0.into(),
        }
    }
}

// --- Text Input ---

pub struct InputStyle;

impl text_input::StyleSheet for InputStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(Color::WHITE),
            border: Border {
                radius: RustloaderTheme::RADIUS_INPUT.into(),
                width: 1.0,
                color: GRAY_200,
            },
            icon_color: RustloaderTheme::TEXT_SECONDARY,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            border: Border {
                color: RustloaderTheme::PRIMARY,
                width: 2.0,
                ..active.border
            },
            ..active
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        RustloaderTheme::TEXT_DISABLED
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        RustloaderTheme::TEXT_PRIMARY
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgba(0.388, 0.400, 0.945, 0.2)
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            background: Background::Color(RustloaderTheme::BG_LIGHT),
            ..active
        }
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        RustloaderTheme::TEXT_DISABLED
    }
}

pub struct InputErrorStyle;

impl text_input::StyleSheet for InputErrorStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(Color::WHITE),
            border: Border {
                radius: RustloaderTheme::RADIUS_INPUT.into(),
                width: 2.0,
                color: RustloaderTheme::ERROR,
            },
            icon_color: RustloaderTheme::ERROR,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        RustloaderTheme::TEXT_DISABLED
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        RustloaderTheme::TEXT_PRIMARY
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgba(0.937, 0.267, 0.267, 0.2)
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            background: Background::Color(RustloaderTheme::BG_LIGHT),
            ..active
        }
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        RustloaderTheme::TEXT_DISABLED
    }
}
