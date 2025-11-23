//! Main view implementation

use crate::gui::app::{Message, DownloadTaskUI};
use crate::gui::components::{url_input, download_item};
use iced::widget::{button, column, container, row, text, rule, scrollable, Space};
use iced::{Element, Length, Alignment};

/// Create the main view
pub fn main_view(
    url_value: &str,
    downloads: &[DownloadTaskUI],
    status_message: &str,
    is_extracting: bool,
) -> Element<'static, Message> {
    // Header
    let header = text("Rustloader")
        .size(32)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(0.3, 0.6, 1.0)));

    // URL input row
    let url_row = url_input(
        url_value,
        Message::UrlInputChanged,
        Message::PasteFromClipboard,
        Message::ClearUrlInput,
    );

    // Button row
    let button_row = row![
        button(if is_extracting { "Extracting..." } else { "▶ Download" })
            .on_press_maybe(if !url_value.is_empty() && !is_extracting {
                Some(Message::DownloadButtonPressed)
            } else {
                None
            })
            .padding(10),
        button("⚙ Settings")
            .on_press(Message::SwitchToSettings)
            .padding(10),
    ]
    .spacing(10)
    .align_items(Alignment::Center);

    // Downloads section
    let downloads_section = if downloads.is_empty() {
        column![
            text("No active downloads")
                .size(16)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5)))
        ]
    } else {
        let mut downloads_col = column![
            text(format!("Active Downloads ({})", downloads.len()))
                .size(20)
        ]
        .spacing(10);

        for task in downloads {
            downloads_col = downloads_col.push(download_item(task));
        }

        downloads_col = downloads_col.push(
            button("Clear Completed")
                .on_press(Message::ClearAllCompleted)
                .padding(8)
        );

        downloads_col
    };

    // Status bar
    let status_bar = text(status_message)
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(0.7, 0.7, 0.7)));

    // Main content
    let content = column![
        header,
        url_row,
        button_row,
        rule::Rule::horizontal(2),
        downloads_section,
        rule::Rule::horizontal(2),
        status_bar,
    ]
    .spacing(20)
    .padding(20)
    .width(Length::Fill);

    // Make the downloads section scrollable
    let scrollable_content = scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill);

    container(scrollable_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
