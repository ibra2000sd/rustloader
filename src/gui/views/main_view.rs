//! Main view implementation - Light Theme

use crate::gui::app::{DownloadTaskUI, Message};
use crate::gui::components::{download_item, url_input};
use iced::widget::{button, column, container, pick_list, row, scrollable, slider, text, Space};
use iced::{Alignment, Element, Length};

/// Create the main view
pub fn main_view(
    url_value: &str,
    downloads: &[DownloadTaskUI],
    _status_message: &str,
    is_extracting: bool,
    url_error: Option<&str>,
    quality: &str,
    segments: usize,
) -> Element<'static, Message> {
    use crate::gui::theme;

    // Hero Input Section
    let hero_section = container(
        column![
            text("Download Video")
                .size(30)
                .style(iced::theme::Text::Color(theme::GRAY_800)),
            url_input(
                url_value,
                Message::UrlInputChanged,
                Message::PasteFromClipboard,
                Message::ClearUrlInput,
                url_error,
            ),
            // Download button row
            row![
                Space::with_width(Length::Fill),
                button(
                    text(if is_extracting {
                        "Extracting..."
                    } else {
                        "Download"
                    })
                    .size(16)
                )
                .on_press_maybe(if !url_value.is_empty() && !is_extracting {
                    Some(Message::DownloadButtonPressed)
                } else {
                    None
                })
                .padding([16, 32])
                .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
            ],
            // Info row with interactive dropdowns
            row![
                // Quality dropdown
                container(
                    column![
                        text("Quality")
                            .size(11)
                            .style(iced::theme::Text::Color(theme::GRAY_500)),
                        pick_list(
                            vec![
                                "Best Available".to_string(),
                                "1080p".to_string(),
                                "720p".to_string(),
                                "480p".to_string()
                            ],
                            Some(quality.to_string()),
                            Message::QualityChanged
                        )
                        .text_size(12)
                        .padding([6, 10])
                        .width(iced::Length::Fixed(140.0)),
                    ]
                    .spacing(4)
                )
                .padding([8, 12])
                .style(iced::theme::Container::Custom(Box::new(InfoTagStyle))),
                // Format tag (static for now - could be made selectable)
                container(
                    column![
                        text("Format")
                            .size(11)
                            .style(iced::theme::Text::Color(theme::GRAY_500)),
                        text("MP4")
                            .size(12)
                            .style(iced::theme::Text::Color(theme::GRAY_800)),
                    ]
                    .spacing(4)
                )
                .padding([8, 12])
                .style(iced::theme::Container::Custom(Box::new(InfoTagStyle))),
                // Segments slider
                container(
                    column![
                        row![
                            text("Segments")
                                .size(11)
                                .style(iced::theme::Text::Color(theme::GRAY_500)),
                            Space::with_width(iced::Length::Fill),
                            text(format!("{}", segments))
                                .size(11)
                                .style(iced::theme::Text::Color(theme::GRAY_800)),
                        ],
                        iced::widget::slider(4..=32, segments as u8, |v| Message::SegmentsChanged(
                            v as usize
                        ))
                        .width(iced::Length::Fixed(120.0)),
                    ]
                    .spacing(4)
                )
                .padding([8, 12])
                .style(iced::theme::Container::Custom(Box::new(InfoTagStyle))),
            ]
            .spacing(12),
        ]
        .spacing(20),
    )
    .padding(32)
    .width(Length::Fill)
    .style(iced::theme::Container::Custom(Box::new(
        theme::GlassContainer,
    )));

    // Downloads section
    let downloads_section: Element<'static, Message> = if downloads.is_empty() {
        container(
            column![
                text("No active downloads")
                    .size(16)
                    .style(iced::theme::Text::Color(theme::GRAY_500)),
                text("Your downloads will appear here")
                    .size(14)
                    .style(iced::theme::Text::Color(theme::GRAY_400)),
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    } else {
        let mut downloads_col = column![row![
            text("Active Downloads")
                .size(24)
                .style(iced::theme::Text::Color(theme::GRAY_800)),
            Space::with_width(Length::Fill),
            button(text("Clear Completed").size(14))
                .on_press(Message::ClearAllCompleted)
                .padding([10, 16])
                .style(iced::theme::Button::Custom(Box::new(
                    theme::SecondaryButton
                ))),
        ]
        .align_items(Alignment::Center)]
        .spacing(24);

        for task in downloads {
            downloads_col = downloads_col.push(download_item(task));
        }

        scrollable(downloads_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced::theme::Scrollable::Custom(Box::new(
                theme::ScrollableStyle,
            )))
            .into()
    };

    // Main content
    column![hero_section, downloads_section,]
        .spacing(32)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([32, 32, 32, 32])
        .into()
}

// Info tag style
struct InfoTagStyle;

impl iced::widget::container::StyleSheet for InfoTagStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        use crate::gui::theme;

        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(theme::GRAY_100)),
            border: iced::Border {
                color: theme::GRAY_200,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        }
    }
}
