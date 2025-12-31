#![allow(clippy::approx_constant)]

//! Custom theme definitions for the application - Light Theme

use iced::widget::{button, container, scrollable, text_input};
use iced::{Background, Border, Color, Gradient, Shadow, Theme, Vector};

// --- Light Color Palette (Comfortable for Eyes) ---

// Background gradients - soft blue to purple
pub const BACKGROUND_START: Color = Color::from_rgb(0.941, 0.976, 1.0); // Sky Blue 50
pub const BACKGROUND_MID: Color = Color::from_rgb(0.878, 0.906, 1.0); // Indigo 50
pub const BACKGROUND_END: Color = Color::from_rgb(0.953, 0.910, 1.0); // Purple 50

// Primary colors - Indigo, Purple, Pink gradient
pub const INDIGO_500: Color = Color::from_rgb(0.388, 0.400, 0.945); // Primary actions
pub const INDIGO_400: Color = Color::from_rgb(0.506, 0.549, 0.973); // Hover state
pub const INDIGO_100: Color = Color::from_rgb(0.878, 0.906, 1.0); // Subtle backgrounds
pub const PURPLE_500: Color = Color::from_rgb(0.545, 0.361, 0.965); // Accent
pub const PINK_500: Color = Color::from_rgb(0.925, 0.282, 0.600); // Accent end

// Success color - Emerald
pub const EMERALD_500: Color = Color::from_rgb(0.063, 0.725, 0.506); // Success state
pub const EMERALD_400: Color = Color::from_rgb(0.204, 0.827, 0.600); // Success gradient

// Danger color - Red
pub const RED_500: Color = Color::from_rgb(0.937, 0.267, 0.267); // Danger state
pub const RED_100: Color = Color::from_rgb(0.996, 0.886, 0.886); // Danger background

// Gray scale for text and borders
pub const GRAY_800: Color = Color::from_rgb(0.122, 0.161, 0.216); // Primary text
pub const GRAY_700: Color = Color::from_rgb(0.216, 0.255, 0.318); // Secondary text
pub const GRAY_600: Color = Color::from_rgb(0.294, 0.333, 0.388); // Tertiary text
pub const GRAY_500: Color = Color::from_rgb(0.420, 0.447, 0.502); // Disabled text
pub const GRAY_400: Color = Color::from_rgb(0.616, 0.639, 0.667); // Placeholder
pub const GRAY_200: Color = Color::from_rgb(0.898, 0.906, 0.922); // Light borders
pub const GRAY_100: Color = Color::from_rgb(0.953, 0.957, 0.965); // Very light bg
pub const GRAY_50: Color = Color::from_rgb(0.976, 0.980, 0.984); // Lightest bg

// White with alpha for glass effects
pub const WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0);
pub const WHITE_70: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.7); // Light glass
pub const WHITE_85: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.85); // Strong glass

// Text colors for compatibility
pub const TEXT_PRIMARY: Color = GRAY_800;
pub const TEXT_SECONDARY: Color = GRAY_600;

// Status colors for compatibility
pub const ACCENT: Color = INDIGO_500;
pub const WARNING: Color = Color::from_rgb(0.961, 0.620, 0.094); // Orange/Amber
pub const SUCCESS: Color = EMERALD_500;
pub const DANGER: Color = RED_500;

// --- Container Styles ---

pub struct MainGradientContainer;

impl container::StyleSheet for MainGradientContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(GRAY_800),
            background: Some(Background::Gradient(Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(2.356)) // 135 degrees
                    .add_stop(0.0, BACKGROUND_START)
                    .add_stop(0.5, BACKGROUND_MID)
                    .add_stop(1.0, BACKGROUND_END),
            ))),
            ..Default::default()
        }
    }
}

pub struct GlassContainer;

impl container::StyleSheet for GlassContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(GRAY_800),
            background: Some(Background::Color(WHITE_85)),
            border: Border {
                color: GRAY_200,
                width: 2.0,
                radius: 24.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.388, 0.400, 0.945, 0.15),
                offset: Vector::new(0.0, 8.0),
                blur_radius: 24.0,
            },
        }
    }
}

pub struct SidebarContainer;

impl container::StyleSheet for SidebarContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(GRAY_700),
            background: Some(Background::Color(WHITE_70)),
            border: Border {
                color: GRAY_200,
                width: 1.0,
                radius: 0.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                offset: Vector::new(2.0, 0.0),
                blur_radius: 8.0,
            },
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
                    .add_stop(0.0, INDIGO_500)
                    .add_stop(0.5, PURPLE_500)
                    .add_stop(1.0, PINK_500),
            ))),
            text_color: WHITE,
            border: Border {
                radius: 16.0.into(),
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
                blur_radius: 8.0,
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
            background: Some(Background::Color(WHITE)),
            text_color: GRAY_700,
            border: Border {
                radius: 12.0.into(),
                color: GRAY_200,
                width: 1.0,
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            shadow_offset: Vector::new(0.0, 0.0),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Background::Color(GRAY_50)),
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
                background: Some(Background::Gradient(Gradient::Linear(
                    iced::gradient::Linear::new(iced::Radians(2.356))
                        .add_stop(0.0, Color::from_rgba(0.388, 0.400, 0.945, 0.1))
                        .add_stop(1.0, Color::from_rgba(0.545, 0.361, 0.965, 0.1)),
                ))),
                text_color: GRAY_700,
                border: Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Self::Inactive => button::Appearance {
                background: None,
                text_color: GRAY_600,
                border: Border {
                    radius: 12.0.into(),
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
                text_color: GRAY_700,
                background: Some(Background::Color(GRAY_100)),
                border: Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

pub struct IconButton;

impl button::StyleSheet for IconButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: GRAY_600,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            text_color: GRAY_800,
            background: Some(Background::Color(GRAY_200)),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub struct DestructiveButton;

impl button::StyleSheet for DestructiveButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: RED_500,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(RED_100)),
            text_color: RED_500,
            border: Border {
                radius: 8.0.into(),
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
            background: Background::Color(WHITE),
            border: Border {
                radius: 16.0.into(),
                width: 2.0,
                color: GRAY_200,
            },
            icon_color: GRAY_500,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            border: Border {
                color: INDIGO_400,
                ..active.border
            },
            ..active
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        GRAY_400
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        GRAY_800
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgba(0.388, 0.400, 0.945, 0.3)
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            background: Background::Color(GRAY_100),
            ..active
        }
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        GRAY_400
    }
}

pub struct InputErrorStyle;

impl text_input::StyleSheet for InputErrorStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(WHITE),
            border: Border {
                radius: 16.0.into(),
                width: 2.0,
                color: RED_500,
            },
            icon_color: RED_500,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        GRAY_400
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        GRAY_800
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgba(0.937, 0.267, 0.267, 0.3)
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            background: Background::Color(GRAY_100),
            ..active
        }
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        GRAY_400
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
                    color: Color::from_rgba(0.388, 0.400, 0.945, 0.3),
                    border: Border {
                        radius: 4.0.into(),
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
                        color: Color::from_rgba(0.388, 0.400, 0.945, 0.5),
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

pub struct ProgressBarStyle;

impl iced::widget::progress_bar::StyleSheet for ProgressBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(INDIGO_100),
            bar: Background::Gradient(Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(0.0))
                    .add_stop(0.0, INDIGO_500)
                    .add_stop(0.5, PURPLE_500)
                    .add_stop(1.0, PINK_500),
            )),
            border_radius: 4.0.into(),
        }
    }
}

pub struct ProgressBarCompleted;

impl iced::widget::progress_bar::StyleSheet for ProgressBarCompleted {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(GRAY_200),
            bar: Background::Gradient(Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(0.0))
                    .add_stop(0.0, EMERALD_400)
                    .add_stop(1.0, EMERALD_500),
            )),
            border_radius: 4.0.into(),
        }
    }
}
