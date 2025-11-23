//! Progress bar component

use iced::widget::{progress_bar as iced_progress_bar, text, column};
use iced::{Element, Length, Color};
use std::time::Duration;

/// Create a progress bar with percentage and ETA
pub fn progress_bar(
    progress: f32,
    eta_seconds: Option<u64>,
) -> Element<'static, crate::gui::app::Message> {
    let bar = iced_progress_bar(0.0..=1.0, progress);

    let eta_text = if let Some(seconds) = eta_seconds {
        let duration = Duration::from_secs(seconds);
        format!("{} remaining", format_duration(duration))
    } else {
        "Calculating...".to_string()
    };

    column![
        bar,
        text(eta_text).size(14),
    ]
    .spacing(5)
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
