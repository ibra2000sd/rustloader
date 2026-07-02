//! Settings view implementation

use iced::widget::{
    button, column, container, pick_list, row, scrollable, slider, text, text_input, toggler, Space,
};
use iced::{Alignment, Element, Length};

/// Create the settings view
pub fn settings_view(
    download_location: &str,
    max_concurrent: usize,
    segments: usize,
    cookies_from_browser: &str,
    detected_browsers: &[String],
    clipboard_monitoring: bool,
) -> Element<'static, crate::gui::app::Message> {
    // Header with back button
    let header = row![
        button(text("← Back").size(16))
            .on_press(crate::gui::app::Message::SwitchToMain)
            .padding([8, 16])
            .style(iced::theme::Button::Custom(Box::new(
                crate::gui::theme::SecondaryButton
            ))),
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
                .style(iced::theme::TextInput::Custom(Box::new(
                    crate::gui::theme::InputStyle
                ))),
            button(text("Browse...").size(14))
                .on_press(crate::gui::app::Message::BrowseDownloadLocation)
                .padding([10, 16])
                .style(iced::theme::Button::Custom(Box::new(
                    crate::gui::theme::SecondaryButton
                ))),
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
                text("Max concurrent downloads")
                    .size(14)
                    .style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
                Space::with_width(Length::Fill),
                text(format!("{}", max_concurrent))
                    .size(14)
                    .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
            ],
            slider(1..=10, max_concurrent as u8, |v| {
                crate::gui::app::Message::MaxConcurrentChanged(v as usize)
            })
            .width(Length::Fill),
        ]
        .spacing(8),
        // Segments per download
        column![
            row![
                text("Segments per download")
                    .size(14)
                    .style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
                Space::with_width(Length::Fill),
                text(format!("{}", segments))
                    .size(14)
                    .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
            ],
            slider(4..=32, segments as u8, |v| {
                crate::gui::app::Message::SegmentsChanged(v as usize)
            })
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

    // Cookies section — for sites that need authentication (e.g. YouTube's
    // "Sign in to confirm you're not a bot"). A dropdown of detected browsers
    // (plus "None") replaces free-text so the value is always valid for yt-dlp.
    const NONE_LABEL: &str = "None";
    let mut cookie_options: Vec<String> = vec![NONE_LABEL.to_string()];
    cookie_options.extend(detected_browsers.iter().cloned());
    // Keep a previously-saved browser selectable even if detection missed it.
    if !cookies_from_browser.is_empty() && !cookie_options.iter().any(|o| o == cookies_from_browser)
    {
        cookie_options.push(cookies_from_browser.to_string());
    }
    let selected = if cookies_from_browser.is_empty() {
        NONE_LABEL.to_string()
    } else {
        cookies_from_browser.to_string()
    };
    let detected_note = if detected_browsers.is_empty() {
        "No browsers detected — choose None, or install/sign in to a browser.".to_string()
    } else {
        format!("Detected: {}.", detected_browsers.join(", "))
    };
    let cookies_section = column![
        text("YouTube / Authenticated Sites")
            .size(16)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
        text("Read cookies from this browser so logged-in / age-restricted videos work. Applies on next launch.")
            .size(13)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
        pick_list(cookie_options, Some(selected), |s| {
            crate::gui::app::Message::CookiesFromBrowserChanged(if s == NONE_LABEL {
                String::new()
            } else {
                s
            })
        })
        .width(Length::Fill)
        .padding(10),
        text(detected_note)
            .size(11)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
    ]
    .spacing(10);

    // Clipboard monitoring section — explicit opt-in (default OFF) because it
    // reads everything the user copies. The plain-language label spells out
    // exactly what it does and what it never does.
    let clipboard_section = column![
        text("Clipboard Monitoring")
            .size(16)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_PRIMARY)),
        text(
            "Watch the clipboard while Rustloader is running. When you copy a web link \
             (http/https), you'll be asked whether to download it — nothing downloads \
             automatically, and clipboard contents are never stored or sent anywhere."
        )
        .size(13)
        .style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
        toggler(
            Some("Detect copied URLs".to_string()),
            clipboard_monitoring,
            crate::gui::app::Message::ClipboardMonitoringToggled,
        )
        .width(Length::Shrink)
        .spacing(8),
        text("Takes effect immediately; Save Settings keeps it for next launch.")
            .size(11)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
    ]
    .spacing(10);

    // Save button
    let save_button = button(text("Save Settings").size(16))
        .on_press(crate::gui::app::Message::SaveSettings)
        .padding([12, 24])
        .width(Length::Fill)
        .style(iced::theme::Button::Custom(Box::new(
            crate::gui::theme::PrimaryButton,
        )));

    // Main content: header pinned at the top, the settings sections in a
    // scroll area, and the Save button pinned at the bottom so it is always
    // visible regardless of window height (previously it sat below the fold).
    let content = column![
        header,
        scrollable(
            container(
                column![
                    download_location_section,
                    performance_section,
                    quality_section,
                    cookies_section,
                    clipboard_section,
                ]
                .spacing(24)
            )
            .padding(24)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(
                crate::gui::theme::GlassContainer
            )))
        )
        .height(Length::Fill),
        save_button,
    ]
    .spacing(24)
    .padding(32)
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(
            crate::gui::theme::MainGradientContainer,
        )))
        .into()
}
