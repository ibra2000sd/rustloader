use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};

use crate::gui::app::{DownloadTaskUI, Message};
use crate::gui::components::{DownloadCard, DownloadCardMessage, DownloadStatus};
use crate::gui::theme::{self, RustloaderTheme};
use crate::utils::config::VideoQuality;

pub fn main_view<'a>(
    url_input_value: &'a str,
    downloads: &'a [DownloadTaskUI],
    status_message: &'a str,
    is_extracting: bool,
    url_error: Option<&'a str>,
    quality: VideoQuality,
    _segments: usize,
) -> Element<'a, Message> {
    // 1. Input Section
    let input_section = {
        let input = text_input("Paste video URL here...", url_input_value)
            .on_input(Message::UrlInputChanged)
            .on_submit(Message::DownloadButtonPressed)
            .padding(12)
            .size(14)
            .width(Length::Fill)
            .style(if url_error.is_some() {
                iced::theme::TextInput::Custom(Box::new(theme::InputErrorStyle))
            } else {
                iced::theme::TextInput::Custom(Box::new(theme::InputStyle))
            });

        let download_btn = button(
            text(if is_extracting {
                "Processing..."
            } else {
                "Download"
            })
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::DownloadButtonPressed)
        .padding(12)
        .width(Length::Fixed(120.0))
        .style(iced::theme::Button::Custom(Box::new(theme::PrimaryButton)));

        let paste_btn = button(
            text("Paste")
                .size(14)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::PasteFromClipboard)
        .padding(12)
        .width(Length::Fixed(80.0))
        .style(iced::theme::Button::Secondary);

        column![
            text("Download Video")
                .size(18)
                .style(iced::theme::Text::Color(RustloaderTheme::TEXT_PRIMARY)),
            Space::with_height(8),
            row![input, paste_btn, download_btn].spacing(10),
            if let Some(error) = url_error {
                text(error)
                    .size(12)
                    .style(iced::theme::Text::Color(RustloaderTheme::ERROR))
            } else {
                text("Supports YouTube, Vimeo, and more")
                    .size(12)
                    .style(iced::theme::Text::Color(RustloaderTheme::TEXT_SECONDARY))
            }
        ]
        .spacing(4)
    };

    // 2. Options Section
    let options_section = {
        let quality_options = vec![
            VideoQuality::Best,
            VideoQuality::Specific("1080".to_string()),
            VideoQuality::Specific("720".to_string()),
            VideoQuality::Specific("480".to_string()),
            VideoQuality::Worst,
        ];

        let quality_picker = pick_list(quality_options, Some(quality.clone()), |q| {
            Message::QualityChanged(q.to_format_string())
        })
        .width(Length::Fixed(150.0))
        .padding(10);

        row![
            text("Quality:")
                .size(14)
                .style(iced::theme::Text::Color(RustloaderTheme::TEXT_SECONDARY)),
            quality_picker,
        ]
        .spacing(12)
        .align_items(Alignment::Center)
    };

    // 3. Status Bar
    let status_bar = container(
        text(status_message)
            .size(12)
            .style(iced::theme::Text::Color(RustloaderTheme::TEXT_SECONDARY)),
    )
    .padding(10)
    .width(Length::Fill);

    // 4. Downloads List
    let downloads_list = if downloads.is_empty() {
        container(
            column![
                text("No active downloads")
                    .size(16)
                    .style(iced::theme::Text::Color(RustloaderTheme::TEXT_SECONDARY)),
                text("Paste a URL above to start")
                    .size(12)
                    .style(iced::theme::Text::Color(RustloaderTheme::TEXT_DISABLED)),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
    } else {
        let list = column(
            downloads
                .iter()
                .map(|task| {
                    let status = match task.status.as_str() {
                        "Downloading" => DownloadStatus::Downloading,
                        "Paused" => DownloadStatus::Paused,
                        "Completed" | "Complete" => DownloadStatus::Complete,
                        "Failed" => DownloadStatus::Failed,
                        _ => DownloadStatus::Queued,
                    };

                    DownloadCard {
                        id: task.id.clone(),
                        title: task.title.clone(),
                        progress: task.progress as f32,
                        speed: if task.speed > 0.0 {
                            format!("{:.1} MB/s", task.speed / 1024.0 / 1024.0)
                        } else {
                            "0.0 MB/s".to_string()
                        },
                        eta_seconds: task.eta_seconds,
                        downloaded_mb: task.downloaded_mb,
                        total_mb: task.total_mb,
                        status,
                        quality_label: "MP4".to_string(),
                    }
                    .view()
                    .map(Message::from)
                })
                .collect::<Vec<_>>(),
        )
        .spacing(16);

        container(
            scrollable(list).style(iced::theme::Scrollable::Custom(Box::new(
                theme::ScrollableStyle,
            ))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
    };

    column![
        container(input_section)
            .padding(24)
            .style(iced::theme::Container::Custom(Box::new(
                theme::GlassContainer
            ))),
        container(options_section).padding(12),
        Space::with_height(12),
        text("Active Downloads")
            .size(14)
            .style(iced::theme::Text::Color(RustloaderTheme::TEXT_SECONDARY)),
        Space::with_height(12),
        downloads_list,
        status_bar,
    ]
    .spacing(0)
    .padding(24)
    .into()
}

// Adapters
impl From<DownloadCardMessage> for Message {
    fn from(msg: DownloadCardMessage) -> Self {
        match msg {
            DownloadCardMessage::Pause(id) => Message::PauseDownload(id),
            DownloadCardMessage::Resume(id) => Message::ResumeDownload(id),
            DownloadCardMessage::Cancel(id) => Message::CancelDownload(id),
            DownloadCardMessage::OpenFolder(id) => Message::OpenDownloadFolder(id),
            DownloadCardMessage::Remove(id) => Message::RemoveCompleted(id),
        }
    }
}
