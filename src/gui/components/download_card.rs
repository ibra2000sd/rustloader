use crate::gui::theme::{self, RustloaderTheme};
use iced::{
    widget::{button, column, container, progress_bar, row, text, Row},
    Alignment, Element, Length,
};

#[derive(Debug, Clone)]
pub struct DownloadCard {
    pub id: String,
    pub title: String,
    pub progress: f32, // 0.0 to 1.0
    pub speed: String,
    pub eta_seconds: Option<u64>,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub status: DownloadStatus,
    pub quality_label: String, // e.g., "1080p"
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Complete,
    Failed,
}

impl DownloadCard {
    pub fn view(self) -> Element<'static, DownloadCardMessage> {
        let status_icon = match self.status {
            DownloadStatus::Queued => "⏳",
            DownloadStatus::Downloading => "⬇️",
            DownloadStatus::Paused => "⏸️",
            DownloadStatus::Complete => "✅",
            DownloadStatus::Failed => "❌",
        };

        let status_text_color = match self.status {
            DownloadStatus::Failed => RustloaderTheme::ERROR,
            DownloadStatus::Complete => RustloaderTheme::SUCCESS,
            DownloadStatus::Paused => RustloaderTheme::WARNING,
            _ => RustloaderTheme::PRIMARY,
        };

        let title_row = row![
            text(status_icon).size(20),
            text(self.title.clone())
                .size(16)
                .style(iced::theme::Text::Color(status_text_color)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let progress_val = self.progress * 100.0;
        let progress_row = row![
            progress_bar(0.0..=100.0, progress_val)
                .height(Length::Fixed(6.0))
                .style(iced::theme::ProgressBar::Custom(Box::new(
                    theme::ProgressBarStyle
                ))),
            text(format!("{:.1}%", progress_val))
                .size(12)
                .style(iced::theme::Text::Color(RustloaderTheme::TEXT_PRIMARY)),
            text(self.speed.clone())
                .size(12)
                .style(iced::theme::Text::Color(RustloaderTheme::SECONDARY)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let eta_str = if let Some(seconds) = self.eta_seconds {
            format_duration(seconds)
        } else {
            "--:--".to_string()
        };

        let info_text = match self.status {
            DownloadStatus::Complete => format!(
                "{} • {:.1} MB • Completed",
                self.quality_label, self.total_mb
            ),
            DownloadStatus::Failed => "Download Failed".to_string(),
            _ => format!(
                "{} • {:.1}/{:.1} MB • ETA: {}",
                self.quality_label, self.downloaded_mb, self.total_mb, eta_str
            ),
        };

        let info_row = row![text(info_text)
            .size(12)
            .style(iced::theme::Text::Color(RustloaderTheme::TEXT_SECONDARY)),]
        .width(Length::Fill);

        let actions = self.render_actions();

        let content = column![
            title_row,
            progress_row,
            row![info_row, actions]
                .align_items(Alignment::Center)
                .spacing(16),
        ]
        .spacing(12)
        .padding(16);

        container(content)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(
                theme::DownloadCardStyle,
            )))
            .into()
    }

    fn render_actions(&self) -> Row<'static, DownloadCardMessage> {
        // defined inline to avoid cloning issue with Box<dyn StyleSheet>
        let btn_style = || iced::theme::Button::Custom(Box::new(theme::ActionButton));
        let dest_style = || iced::theme::Button::Custom(Box::new(theme::DestructiveButton));

        match self.status {
            DownloadStatus::Downloading => {
                row![
                    button(text("⏸").size(14))
                        .on_press(DownloadCardMessage::Pause(self.id.clone()))
                        .style(btn_style()),
                    button(text("⏹").size(14))
                        .on_press(DownloadCardMessage::Cancel(self.id.clone()))
                        .style(dest_style()),
                ]
            }
            DownloadStatus::Paused | DownloadStatus::Queued => {
                row![
                    button(text("▶").size(14))
                        .on_press(DownloadCardMessage::Resume(self.id.clone()))
                        .style(btn_style()),
                    button(text("⏹").size(14))
                        .on_press(DownloadCardMessage::Cancel(self.id.clone()))
                        .style(dest_style()),
                ]
            }
            DownloadStatus::Complete | DownloadStatus::Failed => {
                row![
                    button(text("📁").size(14))
                        .on_press(DownloadCardMessage::OpenFolder(self.id.clone()))
                        .style(btn_style()),
                    button(text("🗑").size(14))
                        .on_press(DownloadCardMessage::Remove(self.id.clone()))
                        .style(dest_style()),
                ]
            }
        }
        .spacing(8)
    }
}

fn format_duration(seconds: u64) -> String {
    let minutes = seconds / 60;
    let rem_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, rem_seconds)
}

#[derive(Debug, Clone)]
pub enum DownloadCardMessage {
    Pause(String),
    Resume(String),
    Cancel(String),
    OpenFolder(String),
    Remove(String),
}
