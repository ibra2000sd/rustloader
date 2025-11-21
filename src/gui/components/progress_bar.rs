//! Progress bar component

use iced::widget::{progress_bar, text};
use iced::{Element, Length, Color};
use std::time::Duration;

/// Create a progress bar with percentage and ETA
pub fn progress_bar(
    progress: f32,
    eta_seconds: Option<u64>,
) -> Element<'static, crate::gui::app::Message> {
    let percentage = (progress * 100.0) as u32;
    let eta_text = if let Some(seconds) = eta_seconds {
        format!("{} remaining", format_duration(Duration::from_secs(seconds)))
    } else {
        "Calculating...".to_string()
    };

    iced::widget::column![
        iced::widget::row![
            text(format!("{}%", percentage)).size(14),
            iced::widget::Space::with_width(Length::Fill),
            text(eta_text).size(14)
        ]
        .width(Length::Fill),
        progress_bar(0.0..=1.0, progress)
            .width(Length::Fill)
            .height(8)
    ]
    .spacing(5)
    .width(Length::Fill)
    .into()
}

/// Format duration as human-readable string
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}
