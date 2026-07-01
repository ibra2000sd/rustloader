//! Download-history view implementation (Shape-3 PR-2)
//!
//! Reads the persisted `downloads` table (F-HIST-001, #34) — a durable log of
//! every download ever started, independent of the live queue. This view is
//! read/delete only: it never mutates the live queue's state, which remains
//! the sole runtime authority for in-flight downloads.

use crate::database::DownloadRecord;
use crate::gui::app::Message;
use crate::gui::components::history_item;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};

/// Create the download-history view.
pub fn history_view(
    history: &[DownloadRecord],
    is_loading: bool,
    load_error: Option<&str>,
) -> Element<'static, Message> {
    use crate::gui::theme;

    let header = row![
        button(text("← Back").size(16))
            .on_press(Message::SwitchToMain)
            .padding([8, 16])
            .style(iced::theme::Button::Custom(Box::new(
                theme::SecondaryButton
            ))),
        Space::with_width(Length::Fill),
        text("History")
            .size(24)
            .style(iced::theme::Text::Color(theme::TEXT_PRIMARY)),
        Space::with_width(Length::Fill),
        button(
            text(if is_loading {
                "Refreshing..."
            } else {
                "Refresh"
            })
            .size(14)
        )
        .on_press_maybe(if is_loading {
            None
        } else {
            Some(Message::RefreshHistory)
        })
        .padding([8, 16])
        .style(iced::theme::Button::Custom(Box::new(
            theme::SecondaryButton
        ))),
    ]
    .spacing(10)
    .align_items(Alignment::Center);

    let body: Element<'static, Message> = if let Some(error) = load_error {
        container(
            column![
                text("Couldn't load download history")
                    .size(16)
                    .style(iced::theme::Text::Color(theme::DANGER)),
                text(error.to_string())
                    .size(13)
                    .style(iced::theme::Text::Color(theme::TEXT_SECONDARY)),
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    } else if history.is_empty() {
        container(
            column![
                text(if is_loading {
                    "Loading history..."
                } else {
                    "No download history yet"
                })
                .size(16)
                .style(iced::theme::Text::Color(theme::GRAY_500)),
                text("Downloads you complete will show up here.")
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
        let mut history_col = column![].spacing(16);
        for record in history {
            history_col = history_col.push(history_item(record));
        }

        scrollable(history_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced::theme::Scrollable::Custom(Box::new(
                theme::ScrollableStyle,
            )))
            .into()
    };

    let content = column![header, body]
        .spacing(24)
        .padding(32)
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(
            theme::MainGradientContainer,
        )))
        .into()
}
