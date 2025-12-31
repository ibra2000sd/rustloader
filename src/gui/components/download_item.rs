//! Download item component

use crate::gui::app::{DownloadTaskUI, Message};
use crate::gui::components::progress_bar;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Color, Element, Length};

/// Create a download item widget
/// Create a download item widget
pub fn download_item(task: &DownloadTaskUI) -> Element<'static, Message> {
    use crate::gui::theme;
    use iced::Theme;

    let status_color = match task.status.as_str() {
        "Downloading" => theme::ACCENT,
        "Paused" | "Pausing..." | "Resuming..." => theme::WARNING,
        "Completed" => theme::SUCCESS,
        "Failed" => theme::DANGER,
        _ => theme::TEXT_SECONDARY,
    };

    let control_buttons = match task.status.as_str() {
        "Downloading" => row![
            button(text("Pause").size(12))
                .on_press(Message::PauseDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(
                    theme::SecondaryButton
                ))),
            button(text("Cancel").size(12))
                .on_press(Message::CancelDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(
                    theme::DestructiveButton
                ))),
        ],

        // ✅ FIX BUG-008: Handle "Pausing...", "Paused", and "Resuming..." states
        "Pausing..." | "Paused" | "Resuming..." => row![
            button(text("Resume").size(12))
                .on_press(Message::ResumeDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
            button(text("Cancel").size(12))
                .on_press(Message::CancelDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(
                    theme::DestructiveButton
                ))),
        ],

        "Completed" => row![
            button(text("Open File").size(12))
                .on_press(Message::OpenFile(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
            button(text("Show in Folder").size(12))
                .on_press(Message::OpenDownloadFolder(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(
                    theme::SecondaryButton
                ))),
            button(text("Remove").size(12))
                .on_press(Message::RemoveCompleted(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(theme::IconButton))),
        ],

        "Failed" => row![
            button(text("Retry").size(12))
                .on_press(Message::RetryDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
            button(text("Remove").size(12))
                .on_press(Message::RemoveCompleted(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(theme::IconButton))),
        ],

        // ✅ FIX BUG-008: Handle "Cancelling..." state
        "Cancelling..." => row![text("Cancelling...")
            .size(12)
            .style(iced::theme::Text::Color(theme::TEXT_SECONDARY)),],

        // Default for any unknown states
        _ => row![button(text("Cancel").size(12))
            .on_press(Message::CancelDownload(task.id.clone()))
            .padding([6, 12])
            .style(iced::theme::Button::Custom(Box::new(
                theme::DestructiveButton
            ))),],
    };

    let speed_text = if task.status == "Completed" {
        "Complete".to_string()
    } else if task.speed > 0.0 {
        format!("{:.1} MB/s", task.speed / 1024.0 / 1024.0)
    } else {
        "0.0 MB/s".to_string()
    };

    let size_text = if task.status == "Completed" && task.total_mb > 0.0 {
        format!("{:.1} MB", task.total_mb)
    } else if task.total_mb > 0.0 {
        format!("{:.1} MB / {:.1} MB", task.downloaded_mb, task.total_mb)
    } else {
        format!("{:.1} MB", task.downloaded_mb)
    };

    let content = column![
        // Title and status
        row![
            text(&task.title)
                .size(16)
                .width(Length::Fill)
                .style(theme::TEXT_PRIMARY),
            text(&task.status).size(12).style(status_color),
        ]
        .spacing(10)
        .align_items(Alignment::Center),
        // Progress bar
        progress_bar(task.progress, task.eta_seconds),
        // Speed and size
        row![
            text(speed_text).size(12).style(theme::TEXT_SECONDARY),
            Space::with_width(Length::Fill),
            text(size_text).size(12).style(theme::TEXT_SECONDARY),
        ]
        .spacing(10)
        .align_items(Alignment::Center),
        // Control buttons
        row![Space::with_width(Length::Fill), control_buttons.spacing(8),]
    ]
    .spacing(12)
    .width(Length::Fill);

    container(content)
        .padding(16)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(
            theme::GlassContainer,
        )))
        .into()
}
