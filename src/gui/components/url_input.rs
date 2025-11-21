//! URL input component

use iced::widget::{button, text_input, row, tooltip};
use iced::{Element, Length, Alignment};
use crate::gui::app::Message;

/// Create a URL input field with paste button
pub fn url_input(
    value: &str,
    on_change: impl Fn(String) -> Message + 'static,
    on_paste: Message,
    on_clear: Message,
) -> Element<'static, Message> {
    row![
        text_input("Paste video URL here...", value)
            .on_input(on_change)
            .padding(10)
            .width(Length::Fill),
        tooltip(
            button("📋")
                .on_press(on_paste)
                .padding([6, 10]),
            "Paste from clipboard",
            tooltip::Position::Bottom,
        ),
        button("Clear")
            .on_press(on_clear)
            .padding([6, 10]),
    ]
    .spacing(10)
    .align_items(Alignment::Center)
    .into()
}
