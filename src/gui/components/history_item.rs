//! Download-history row component (Shape-3 PR-2)

use crate::database::DownloadRecord;
use crate::gui::app::Message;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

/// Row title: the download's title, falling back to its URL when the title
/// is empty (e.g. a record saved before extraction resolved a real title).
pub fn history_row_title(record: &DownloadRecord) -> String {
    if record.title.is_empty() {
        record.url.clone()
    } else {
        record.title.clone()
    }
}

/// Human-readable file size, or an explicit "unknown" label rather than a
/// misleading "0.0 MB" when the size was never recorded.
pub fn history_row_size_text(file_size: Option<u64>) -> String {
    file_size
        .map(|bytes| format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)))
        .unwrap_or_else(|| "Unknown size".to_string())
}

/// The timestamp shown on a row: prefer `completed_at` (when the download
/// actually finished) over `created_at` (when it was queued) — that's the
/// more useful date for a history list, and it's what most rows will have
/// once they reach a terminal state.
pub fn history_row_timestamp(record: &DownloadRecord) -> String {
    record
        .completed_at
        .unwrap_or(record.created_at)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Create a single row for a persisted download-history record.
pub fn history_item(record: &DownloadRecord) -> Element<'static, Message> {
    use crate::gui::theme;

    let status_color = match record.status.as_str() {
        "Completed" => theme::SUCCESS,
        "Failed" => theme::DANGER,
        "Downloading" => theme::ACCENT,
        "Paused" | "Queued" => theme::WARNING,
        _ => theme::TEXT_SECONDARY,
    };

    let title = history_row_title(record);
    let size_text = history_row_size_text(record.file_size);
    let timestamp = history_row_timestamp(record);

    let title_row = row![
        text(title)
            .size(16)
            .width(Length::Fill)
            .style(theme::TEXT_PRIMARY),
        text(&record.status).size(12).style(status_color),
    ]
    .spacing(10)
    .align_items(Alignment::Center);

    let mut content = column![title_row];

    if let Some(error) = &record.error_message {
        content = content.push(
            text(format!("✕ {error}"))
                .size(12)
                .style(iced::theme::Text::Color(theme::DANGER)),
        );
    }

    content = content
        .push(
            text(record.output_path.display().to_string())
                .size(12)
                .style(iced::theme::Text::Color(theme::TEXT_SECONDARY)),
        )
        .push(
            row![
                text(size_text).size(12).style(theme::TEXT_SECONDARY),
                Space::with_width(Length::Fill),
                text(timestamp).size(12).style(theme::TEXT_SECONDARY),
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .push(
            row![
                Space::with_width(Length::Fill),
                button(text("Show in Folder").size(12))
                    .on_press(Message::OpenHistoryFolder(record.id.clone()))
                    .padding([6, 12])
                    .style(iced::theme::Button::Custom(Box::new(
                        theme::SecondaryButton
                    ))),
                button(text("Remove").size(12))
                    .on_press(Message::RemoveFromHistory(record.id.clone()))
                    .padding([6, 12])
                    .style(iced::theme::Button::Custom(Box::new(theme::IconButton)))
            ]
            .spacing(8),
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn sample_record() -> DownloadRecord {
        DownloadRecord {
            id: "id-1".to_string(),
            url: "https://example.com/video".to_string(),
            title: "My Video".to_string(),
            output_path: PathBuf::from("/tmp/out.mp4"),
            file_size: Some(2 * 1024 * 1024),
            status: "Completed".to_string(),
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    #[test]
    fn title_uses_the_video_title_when_present() {
        let record = sample_record();
        assert_eq!(history_row_title(&record), "My Video");
    }

    #[test]
    fn title_falls_back_to_url_when_title_is_empty() {
        let mut record = sample_record();
        record.title = String::new();
        assert_eq!(history_row_title(&record), "https://example.com/video");
    }

    #[test]
    fn size_text_formats_known_size_in_mb() {
        assert_eq!(history_row_size_text(Some(2 * 1024 * 1024)), "2.0 MB");
    }

    #[test]
    fn size_text_is_explicit_about_unknown_size() {
        // Must not read as "0.0 MB" -- that would misleadingly claim a known,
        // empty file rather than "we never recorded a size".
        assert_eq!(history_row_size_text(None), "Unknown size");
    }

    #[test]
    fn timestamp_prefers_completed_at_over_created_at() {
        let mut record = sample_record();
        record.created_at = "2026-01-01T00:00:00Z".parse().unwrap();
        record.completed_at = Some("2026-06-15T12:30:00Z".parse().unwrap());
        assert_eq!(history_row_timestamp(&record), "2026-06-15 12:30");
    }

    #[test]
    fn timestamp_falls_back_to_created_at_when_not_completed() {
        let mut record = sample_record();
        record.created_at = "2026-01-01T09:15:00Z".parse().unwrap();
        record.completed_at = None;
        assert_eq!(history_row_timestamp(&record), "2026-01-01 09:15");
    }
}
