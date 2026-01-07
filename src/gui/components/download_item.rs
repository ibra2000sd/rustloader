//! Download item component

use crate::gui::app::{DownloadTaskUI, FailureCategory, Message};
use crate::gui::components::progress_bar;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Color, Element, Length};
use std::time::Duration;

/// Stall detection threshold (30 seconds without progress)
const STALL_THRESHOLD_SECS: u64 = 30;

/// Create a download item widget
pub fn download_item(task: &DownloadTaskUI) -> Element<'static, Message> {
    use crate::gui::theme;
    use iced::Theme;

    // v0.6.0: Stall detection - Downloading with no progress for STALL_THRESHOLD_SECS
    let is_stalled = task.status == "Downloading" 
        && task.last_progress_at.elapsed() > Duration::from_secs(STALL_THRESHOLD_SECS);

    // Determine display status (may differ from backend status)
    let display_status = if is_stalled {
        "⚠ Stalled".to_string()
    } else if task.was_resumed_after_failure && task.status == "Downloading" {
        "Retrying...".to_string()
    } else {
        task.status.clone()
    };

    let status_color = match task.status.as_str() {
        "Downloading" if is_stalled => theme::WARNING,
        "Downloading" => theme::ACCENT,
        "Paused" | "Pausing..." | "Resuming..." => theme::WARNING,
        "Completed" => theme::SUCCESS,
        "Failed" => theme::DANGER,
        _ => theme::TEXT_SECONDARY,
    };

    // v0.7.0: Stalled task controls
    let control_buttons = if is_stalled {
        row![
            button(text("Restart").size(12))
                .on_press(Message::RestartStalled(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
            button(text("Cancel").size(12))
                .on_press(Message::CancelDownload(task.id.clone()))
                .padding([6, 12])
                .style(iced::theme::Button::Custom(Box::new(
                    theme::DestructiveButton
                ))),
        ]
    } else {
        match task.status.as_str() {
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

            // v0.7.0: Enhanced Failed controls with Reset
            "Failed" => row![
                button(text("Retry").size(12))
                    .on_press(Message::RetryDownload(task.id.clone()))
                    .padding([6, 12])
                    .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton))),
                button(text("Reset").size(12))
                    .on_press(Message::ResetTask(task.id.clone()))
                    .padding([6, 12])
                    .style(iced::theme::Button::Custom(Box::new(theme::SecondaryButton))),
                button(text("Remove").size(12))
                    .on_press(Message::RemoveCompleted(task.id.clone()))
                    .padding([6, 12])
                    .style(iced::theme::Button::Custom(Box::new(theme::IconButton))),
            ],

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
        }
    };

    let speed_text = if task.status == "Completed" {
        "Complete".to_string()
    } else if is_stalled {
        "0.0 MB/s (stalled)".to_string()
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

    let title_row = row![
        text(&task.title)
            .size(16)
            .width(Length::Fill)
            .style(theme::TEXT_PRIMARY),
        text(&display_status).size(12).style(status_color),
    ]
    .spacing(10)
    .align_items(Alignment::Center);

    let mut content = column![
        title_row,
    ];

    // v0.7.0: Stall warning with guidance
    if is_stalled {
        content = content.push(
            text("⚠ Download appears stalled. Try restarting or canceling.")
                .size(11)
                .style(iced::theme::Text::Color(theme::WARNING)),
        );
    }

    // v0.7.0: Enhanced error display with failure classification and recovery hint
    if task.status == "Failed" && !task.error_dismissed {
        let error_msg = task.error_message.as_deref().unwrap_or("Unknown error");
        let category = FailureCategory::from_error(error_msg);
        let recovery_hint = category.recovery_hint();
        
        let retry_note = if task.was_resumed_after_failure {
            " (Previously retried)"
        } else {
            ""
        };
        
        // Error message
        content = content.push(
            text(format!("✕ Error: {}{}", error_msg, retry_note))
                .size(12)
                .style(iced::theme::Text::Color(theme::DANGER)),
        );
        
        // Recovery hint
        content = content.push(
            row![
                text(format!("💡 {}", recovery_hint))
                    .size(11)
                    .style(iced::theme::Text::Color(theme::TEXT_SECONDARY)),
                Space::with_width(Length::Fill),
                button(text("Dismiss").size(10))
                    .on_press(Message::DismissError(task.id.clone()))
                    .padding([4, 8])
                    .style(iced::theme::Button::Custom(Box::new(theme::IconButton))),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        );
    }
    
    let is_active = task.status == "Downloading" && !is_stalled;
    
    content = content
        .push(progress_bar(task.progress, task.eta_seconds, is_active, is_stalled))
        .push(
            row![
                text(speed_text).size(12).style(theme::TEXT_SECONDARY),
                Space::with_width(Length::Fill),
                text(size_text).size(12).style(theme::TEXT_SECONDARY),
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .push(row![Space::with_width(Length::Fill), control_buttons.spacing(8),])
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
