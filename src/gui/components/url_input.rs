//! URL input component

use iced::widget::{button, text_input, row, tooltip, text, column};
use iced::{Element, Length, Alignment};
use crate::gui::app::Message;

/// Create a URL input field with paste button and optional error message
pub fn url_input(
    value: &str,
    on_change: impl Fn(String) -> Message + 'static,
    on_paste: Message,
    on_clear: Message,
    error_message: Option<&str>,
) -> Element<'static, Message> {
    use crate::gui::theme;
    use iced::Theme;

    let input_row = row![
        text_input("Paste video URL here...", value)
            .on_input(on_change)
            .padding(15)
            .width(Length::Fill)
            .style(if error_message.is_some() {
                iced::theme::TextInput::Custom(Box::new(theme::InputErrorStyle))
            } else {
                iced::theme::TextInput::Custom(Box::new(theme::InputStyle))
            }),
        tooltip(
            button(text("Paste").size(14))
                .on_press(on_paste)
                .padding([8, 12])
                .style(iced::theme::Button::Custom(Box::new(theme::IconButton))),
            "Paste from clipboard",
            tooltip::Position::Bottom,
        ),
        button(text("Clear").size(14))
            .on_press(on_clear)
            .padding([8, 12])
            .style(iced::theme::Button::Custom(Box::new(theme::IconButton))),
    ]
    .spacing(12)
    .align_items(Alignment::Center);

    if let Some(error) = error_message {
        column![
            input_row,
            row![
                text("Warning: ").size(14).style(theme::DANGER),
                text(error).size(14).style(iced::theme::Text::Color(theme::DANGER)),
            ]
            .spacing(4)
        ]
        .spacing(8)
        .into()
    } else {
        input_row.into()
    }
}
