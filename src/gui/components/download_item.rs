//! Download item component

use crate::gui::app::{Message, DownloadTaskUI};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length, Alignment, Color};
use crate::gui::components::progress_bar;

/// Create a download item widget
pub fn download_item(task: &DownloadTaskUI) -> Element<'static, Message> {
    let status_color = match task.status.as_str() {
        "Downloading" => Color::from_rgb(0.2, 0.7, 1.0),
        "Paused" => Color::from_rgb(0.8, 0.8, 0.2),
        "Completed" => Color::from_rgb(0.2, 0.8, 0.3),
        "Failed" => Color::from_rgb(0.9, 0.2, 0.2),
        _ => Color::from_rgb(0.7, 0.7, 0.7),
    };

    let control_buttons = match task.status.as_str() {
        "Downloading" => row![
            button("⏸ Pause")
                .on_press(Message::PauseDownload(task.id.clone()))
                .padding([6, 12]),
            button("✕ Cancel")
                .on_press(Message::CancelDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Destructive),
        ],
        "Paused" => row![
            button("▶ Resume")
                .on_press(Message::ResumeDownload(task.id.clone()))
                .padding([6, 12]),
            button("✕ Cancel")
                .on_press(Message::CancelDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Destructive),
        ],
        "Completed" => row![
            button("📂 Open")
                .on_press(Message::OpenDownloadFolder(task.id.clone()))
                .padding([6, 12]),
            button("✕ Remove")
                .on_press(Message::RemoveCompleted(task.id.clone()))
                .padding([6, 12])
        ],
        "Failed" => row![
            button("🔄 Retry")
                .on_press(Message::RetryDownload(task.id.clone()))
                .padding([6, 12]),
            button("✕ Remove")
                .on_press(Message::RemoveCompleted(task.id.clone()))
                .padding([6, 12])
        ],
        _ => row![],
    };

    let speed_text = if task.speed > 0.0 {
        format!("{:.1} MB/s", task.speed / 1024.0 / 1024.0)
    } else {
        "0.0 MB/s".to_string()
    };

    let size_text = if task.total_mb > 0.0 {
        format!("{:.1} MB / {:.1} MB", task.downloaded_mb, task.total_mb)
    } else {
        "0.0 MB".to_string()
    };

    let content = column![
        // Title and status
        row![
            text(&task.title).size(16).width(Length::Fill),
            text(&task.status)
                .size(14)
                .style(status_color),
        ]
        .spacing(10)
        .align_items(Alignment::Center),

        // Progress bar
        progress_bar(task.progress, task.eta_seconds),

        // Speed and size
        row![
            text(speed_text).size(14),
            Space::with_width(Length::Fill),
            text(size_text).size(14),
        ]
        .spacing(10)
        .align_items(Alignment::Center),

        // Control buttons
        control_buttons.spacing(8),
    ]
    .spacing(10)
    .width(Length::Fill);

    container(content)
        .padding(15)
        .width(Length::Fill)
        .style(iced::theme::Container::Box)
        .into()
}
