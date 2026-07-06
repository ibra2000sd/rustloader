//! Main view implementation - Light Theme

use crate::gui::app::{DownloadTaskUI, Message};
use crate::gui::components::{download_item, url_input};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, slider, text, Space,
};
use iced::{Alignment, Element, Length};

/// Create the main view
#[allow(clippy::too_many_arguments)] // Mirrors the app-state fields it renders
pub fn main_view(
    url_value: &str,
    downloads: &[DownloadTaskUI],
    _status_message: &str,
    is_extracting: bool,
    url_error: Option<&str>,
    quality: &str,
    output_format: &str,
    insecure_tls: bool,
    segments: usize,
    detected_url: Option<&str>,
    update_available: Option<&str>,
) -> Element<'static, Message> {
    use crate::gui::theme;

    // Hero Input Section
    let hero_section = container(
        column![
            text("Download Video")
                .size(30)
                .style(iced::theme::Text::Color(theme::FG_1)),
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
                            .style(iced::theme::Text::Color(theme::FG_3)),
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
                // Format selector (B-GUI-005 — used to be a dead static
                // "MP4" label while files kept the source container)
                container(
                    column![
                        text("Format")
                            .size(11)
                            .style(iced::theme::Text::Color(theme::FG_3)),
                        pick_list(
                            crate::utils::OutputFormat::ALL
                                .iter()
                                .map(|f| f.label().to_string())
                                .collect::<Vec<_>>(),
                            Some(output_format.to_string()),
                            Message::OutputFormatChanged
                        )
                        .text_size(12)
                        .padding([6, 10])
                        .width(iced::Length::Fixed(140.0)),
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
                                .style(iced::theme::Text::Color(theme::FG_3)),
                            Space::with_width(iced::Length::Fill),
                            text(format!("{}", segments))
                                .size(11)
                                .font(theme::FONT_MONO)
                                .style(iced::theme::Text::Color(theme::FG_1)),
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
            // Per-download TLS escape hatch (first-use feedback, 2026-07-06:
            // a CDN with a hostname-mismatched cert). One-shot and honestly
            // labelled unsafe — never a persisted or global setting, and
            // bridge-initiated downloads ignore it (see app.rs).
            checkbox(
                "Ignore certificate errors (unsafe, next download only)",
                insecure_tls
            )
            .on_toggle(Message::InsecureTlsToggled)
            .text_size(12)
            .size(16)
            .spacing(8),
            // Honest trade-off hint for the selected format (B-GUI-005):
            // conversions happen after download via ffmpeg; MP4 on YouTube
            // means remuxing the (usually AV1/VP9) best streams, and an
            // impossible remux keeps the original container instead of
            // failing.
            format_hint(output_format),
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
                    .style(iced::theme::Text::Color(theme::FG_2)),
                text("Your downloads will appear here")
                    .size(14)
                    .style(iced::theme::Text::Color(theme::FG_3)),
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
                .style(iced::theme::Text::Color(theme::FG_1)),
            Space::with_width(Length::Fill),
            button(text("Resume All").size(14))
                .on_press(Message::ResumeAll)
                .padding([10, 16])
                .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
            button(text("Clear Completed").size(14))
                .on_press(Message::ClearAllCompleted)
                .padding([10, 16])
                .style(iced::theme::Button::Custom(Box::new(
                    theme::SecondaryButton
                ))),
        ]
        .spacing(12)
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

    // Clipboard-detected URL banner (only while clipboard monitoring has a
    // pending candidate): a non-intrusive confirm/dismiss row — downloading
    // only ever starts from the explicit "Download" press.
    let clipboard_banner = detected_url.map(|url| {
        container(
            row![
                column![
                    text("Copied link detected — download it?")
                        .size(14)
                        .style(iced::theme::Text::Color(theme::FG_1)),
                    text(url.to_string())
                        .size(12)
                        .font(theme::FONT_MONO)
                        .shaping(theme::SHAPING_CONTENT) // URLs can carry non-ASCII (IRIs)
                        .style(iced::theme::Text::Color(theme::FG_3)),
                ]
                .spacing(4)
                .width(Length::Fill),
                button(text("Download").size(14))
                    .on_press_maybe(if is_extracting {
                        None
                    } else {
                        Some(Message::ConfirmDetectedUrl)
                    })
                    .padding([8, 16])
                    .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
                button(text("Dismiss").size(14))
                    .on_press(Message::DismissDetectedUrl)
                    .padding([8, 16])
                    .style(iced::theme::Button::Custom(Box::new(
                        theme::SecondaryButton
                    ))),
            ]
            .spacing(12)
            .align_items(Alignment::Center),
        )
        .padding(16)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(
            theme::GlassContainer,
        )))
    });

    // Update-available banner (launch-time check, utils::update_check):
    // same non-intrusive confirm/dismiss shape as the clipboard banner.
    // Download only opens the release page in the browser — no auto-install.
    let update_banner = update_available.map(|version| {
        container(
            row![
                column![
                    text(format!("Update available: v{version}"))
                        .size(14)
                        .style(iced::theme::Text::Color(theme::FG_1)),
                    text("Download opens the release page in your browser.")
                        .size(12)
                        .style(iced::theme::Text::Color(theme::FG_3)),
                ]
                .spacing(4)
                .width(Length::Fill),
                button(text("Download").size(14))
                    .on_press(Message::OpenUpdateDownloadPage)
                    .padding([8, 16])
                    .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
                button(text("Skip this version").size(14))
                    .on_press(Message::SkipUpdateVersion)
                    .padding([8, 16])
                    .style(iced::theme::Button::Custom(Box::new(
                        theme::SecondaryButton
                    ))),
                button(text("Dismiss").size(14))
                    .on_press(Message::DismissUpdateBanner)
                    .padding([8, 16])
                    .style(iced::theme::Button::Custom(Box::new(
                        theme::SecondaryButton
                    ))),
            ]
            .spacing(12)
            .align_items(Alignment::Center),
        )
        .padding(16)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(
            theme::GlassContainer,
        )))
    });

    // Main content
    let mut content = column![hero_section]
        .spacing(32)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([32, 32, 32, 32]);
    if let Some(banner) = update_banner {
        content = content.push(banner);
    }
    if let Some(banner) = clipboard_banner {
        content = content.push(banner);
    }
    content.push(downloads_section).into()
}

// Info tag style
struct InfoTagStyle;

impl iced::widget::container::StyleSheet for InfoTagStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        use crate::gui::theme;

        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(theme::BG_1)),
            border: iced::Border {
                color: theme::BORDER_HAIRLINE,
                width: 1.0,
                radius: theme::RADIUS_CONTROL.into(),
            },
            ..Default::default()
        }
    }
}

/// One-line explanation of what the selected output format will do, shown
/// under the controls row. Empty for the no-conversion default.
fn format_hint(output_format: &str) -> Element<'static, Message> {
    use crate::gui::theme;
    use crate::utils::OutputFormat;

    let hint = match OutputFormat::from_label(output_format) {
        Some(OutputFormat::Best) | None => "",
        Some(OutputFormat::Mp4) | Some(OutputFormat::Webm) | Some(OutputFormat::Mkv) => {
            "Container is changed losslessly after download (no re-encode); \
             if the source codecs don't fit, the original container is kept."
        }
        Some(_) => "Audio only — extracts and converts the sound track; no video.",
    };
    if hint.is_empty() {
        Space::with_height(iced::Length::Fixed(0.0)).into()
    } else {
        text(hint)
            .size(11)
            .style(iced::theme::Text::Color(theme::FG_3))
            .into()
    }
}
