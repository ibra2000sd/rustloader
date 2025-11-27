//! Settings view implementation

use iced::widget::{button, column, container, pick_list, row, slider, text, text_input, Space};
use iced::{Element, Length, Alignment};

/// Create the settings view
pub fn settings_view(
    download_location: &str,
    max_concurrent: usize,
    segments: usize,
) -> Element<'static, crate::gui::app::Message> {
    // Header with back button
    let header = row![
        button(text("← Back").size(16))
            .on_press(crate::gui::app::Message::SwitchToMain)
            .padding([8, 16])
            .style(iced::theme::Button::Custom(Box::new(crate::gui::theme::SecondaryButton))),
        Space::with_width(Length::Fill),
        text("Settings")
            .size(24)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
        Space::with_width(Length::Fill),
        Space::with_width(Length::Fixed(80.0)), // Balance the back button
    ]
    .spacing(10)
    .align_items(Alignment::Center);

    // Download location section
    let download_location_section = column![
        text("Download Location")
            .size(16)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
        row![
            text_input("", download_location)
                .on_input(crate::gui::app::Message::DownloadLocationChanged)
                .padding(12)
                .width(Length::Fill)
                .style(iced::theme::TextInput::Custom(Box::new(crate::gui::theme::InputStyle))),
            button(text("Browse...").size(14))
                .on_press(crate::gui::app::Message::BrowseDownloadLocation)
                .padding([10, 16])
                .style(iced::theme::Button::Custom(Box::new(crate::gui::theme::SecondaryButton))),
        ]
        .spacing(10)
        .align_items(Alignment::Center),
    ]
    .spacing(10);

    // Performance section
    let performance_section = column![
        text("Performance")
            .size(16)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),

        // Max concurrent downloads
        column![
            row![
                text("Max concurrent downloads").size(14).style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
                Space::with_width(Length::Fill),
                text(format!("{}", max_concurrent)).size(14).style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
            ],
            slider(1..=10, max_concurrent as u8, |v| crate::gui::app::Message::MaxConcurrentChanged(v as usize))
                .width(Length::Fill),
        ]
        .spacing(8),

        // Segments per download
        column![
            row![
                text("Segments per download").size(14).style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
                Space::with_width(Length::Fill),
                text(format!("{}", segments)).size(14).style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
            ],
            slider(4..=32, segments as u8, |v| crate::gui::app::Message::SegmentsChanged(v as usize))
                .width(Length::Fill),
        ]
        .spacing(8),
    ]
    .spacing(20);

    // Quality section
    let quality_options = vec!["Best Available", "1080p", "720p", "480p"];
    let quality_section = column![
        text("Quality")
            .size(16)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
        pick_list(
            quality_options,
            Some("Best Available"), // Simplified for UI demo, real app would match current
            |quality| crate::gui::app::Message::QualityChanged(quality.to_string()),
        )
        .width(Length::Fill)
        .padding(10),
    ]
    .spacing(10);

    // Save button
    let save_button = button(text("Save Settings").size(16))
        .on_press(crate::gui::app::Message::SaveSettings)
        .padding([12, 24])
        .width(Length::Fill)
        .style(iced::theme::Button::Custom(Box::new(crate::gui::theme::PrimaryButton)));

    // Main content
    let content = column![
        header,
        container(
            column![
                download_location_section,
                performance_section,
                quality_section,
            ]
            .spacing(24)
        )
        .padding(24)
        .style(iced::theme::Container::Custom(Box::new(crate::gui::theme::GlassContainer))),
        Space::with_height(Length::Fill),
        save_button,
    ]
    .spacing(24)
    .padding(32)
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(crate::gui::theme::MainGradientContainer)))
        .into()
}
