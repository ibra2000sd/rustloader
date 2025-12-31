//! Progress bar component

use iced::widget::{column, progress_bar as iced_progress_bar, text};
use iced::{Color, Element, Length};
use std::time::Duration;

/// Create a progress bar with percentage and ETA
pub fn progress_bar(
    progress: f32,
    eta_seconds: Option<u64>,
) -> Element<'static, crate::gui::app::Message> {
    let style = if progress >= 1.0 {
        iced::theme::ProgressBar::Custom(Box::new(crate::gui::theme::ProgressBarCompleted))
    } else {
        iced::theme::ProgressBar::Custom(Box::new(crate::gui::theme::ProgressBarStyle))
    };

    let bar = iced_progress_bar(0.0..=1.0, progress).style(style);

    let eta_text = if progress >= 1.0 {
        "Completed".to_string()
    } else if let Some(seconds) = eta_seconds {
        if seconds == 0 {
            "Almost done...".to_string()
        } else {
            let duration = Duration::from_secs(seconds);
            format!("{} remaining", format_duration(duration))
        }
    } else {
        "Calculating...".to_string()
    };

    column![
        bar,
        text(eta_text)
            .size(12)
            .style(iced::theme::Text::Color(crate::gui::theme::TEXT_SECONDARY)),
    ]
    .spacing(6)
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
        let seconds = total_seconds % 60;
        format!("{}h {}m {}s", hours, minutes, seconds)
    }
}
