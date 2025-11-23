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
        button("← Back")
            .on_press(crate::gui::app::Message::SwitchToMain)
            .padding([6, 12]),
        text("Settings")
            .size(24)
            .width(Length::Fill),
        Space::with_width(Length::Fixed(80.0)), // Balance the back button
    ]
    .spacing(10)
    .align_items(Alignment::Center);

    // Download location section
    let download_location_section = column![
        text("Download Location:").size(18),
        row![
            text_input("", download_location)
                .on_input(crate::gui::app::Message::DownloadLocationChanged)
                .padding(10)
                .width(Length::Fill),
            button("Browse...")
                .on_press(crate::gui::app::Message::BrowseDownloadLocation)
                .padding([6, 12]),
        ]
        .spacing(10)
        .align_items(Alignment::Center),
    ]
    .spacing(10);

    // Performance section
    let performance_section = column![
        text("Performance:").size(18),

        // Max concurrent downloads
        column![
            text(format!("Max concurrent downloads: {}", max_concurrent)),
            slider(1..=10, max_concurrent as u8, |v| crate::gui::app::Message::MaxConcurrentChanged(v as usize))
                .width(Length::Fill),
        ]
        .spacing(5),

        // Segments per download
        column![
            text(format!("Segments per download: {}", segments)),
            slider(4..=32, segments as u8, |v| crate::gui::app::Message::SegmentsChanged(v as usize))
                .width(Length::Fill),
        ]
        .spacing(5),
    ]
    .spacing(15);

    // Quality section
    let quality_options = vec!["Best Available", "1080p", "720p", "480p"];
    let quality_section = column![
        text("Quality:").size(18),
        pick_list(
            quality_options,
            Some(&"Best Available"),
            |quality| crate::gui::app::Message::QualityChanged(quality.to_string()),
        )
        .width(Length::Fill),
    ]
    .spacing(10);

    // Save button
    let save_button = button("Save Settings")
        .on_press(crate::gui::app::Message::SaveSettings)
        .padding([10, 20])
        .width(Length::Fill);

    // Main content
    let content = column![
        header,
        download_location_section,
        performance_section,
        quality_section,
        Space::with_height(Length::Fill),
        save_button,
    ]
    .spacing(20)
    .padding(20)
    .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
