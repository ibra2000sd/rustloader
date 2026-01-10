use iced::widget::{button, column, container, scrollable, text, Column, Row};
use iced::{Alignment, Element, Length};

use crate::extractor::models::{QualityPreset, VideoFormat, VideoInfo};
use crate::gui::theme;

/// Messages from the format selector
#[derive(Debug, Clone)]
pub enum FormatSelectorMessage {
    /// User selected a quality preset
    PresetSelected(QualityPreset),
    /// User selected a specific format
    FormatSelected(String),
    /// User confirmed selection
    Confirm,
    /// User cancelled
    Cancel,
}

/// State for the format selector
#[derive(Debug, Clone)]
pub struct FormatSelector {
    /// The video being configured
    pub video: VideoInfo,
    /// Currently selected preset
    pub selected_preset: QualityPreset,
    /// Currently selected format ID
    pub selected_format_id: Option<String>,
    /// Whether to show all formats or just common ones
    pub show_all_formats: bool,
}

impl FormatSelector {
    /// Create a new format selector for a video
    pub fn new(video: VideoInfo) -> Self {
        // Default to best format
        let selected_format_id = video.best_format_id.clone();

        Self {
            video,
            selected_preset: QualityPreset::Best,
            selected_format_id,
            show_all_formats: false,
        }
    }

    /// Handle messages
    pub fn update(&mut self, message: FormatSelectorMessage) {
        match message {
            FormatSelectorMessage::PresetSelected(preset) => {
                self.selected_preset = preset;

                // Auto-select best format for preset
                self.selected_format_id = match preset {
                    QualityPreset::Best => self.video.best_format_id.clone(),
                    QualityPreset::AudioOnly => self
                        .video
                        .audio_formats()
                        .first()
                        .map(|f| f.format_id.clone()),
                    _ => {
                        if let Some(max_height) = preset.max_height() {
                            self.video
                                .best_format_for_quality(max_height)
                                .map(|f| f.format_id.clone())
                        } else {
                            self.video.best_format_id.clone()
                        }
                    }
                };
            }
            FormatSelectorMessage::FormatSelected(format_id) => {
                self.selected_format_id = Some(format_id);
            }
            _ => {}
        }
    }

    /// Get the selected format
    pub fn selected_format(&self) -> Option<&VideoFormat> {
        self.selected_format_id
            .as_ref()
            .and_then(|id| self.video.get_format(id))
    }

    /// Render the component
    pub fn view(&self) -> Element<FormatSelectorMessage> {
        let title = text(&self.video.title).size(18);

        // Quality presets row
        let presets_row = Row::with_children(
            QualityPreset::all()
                .iter()
                .map(|preset| {
                    let is_selected = self.selected_preset == *preset;

                    let style = if is_selected {
                        iced::theme::Button::Primary
                    } else {
                        iced::theme::Button::Secondary
                    };

                    button(text(preset.label()))
                        .padding([8, 16])
                        .style(style)
                        .on_press(FormatSelectorMessage::PresetSelected(*preset))
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(8)
        .align_items(Alignment::Center);

        // Format list
        let format_list = self.render_format_list();

        // Selected format summary
        let summary = if let Some(format) = self.selected_format() {
            text(format!("Selected: {}", format.display_label())).size(14)
        } else {
            text("No format selected").size(14)
        };

        // Action buttons
        let actions = Row::new()
            .spacing(12)
            .align_items(Alignment::Center)
            .push(
                button(text("Cancel"))
                    .padding([10, 20])
                    .style(iced::theme::Button::Secondary)
                    .on_press(FormatSelectorMessage::Cancel),
            )
            .push(
                button(text("Download"))
                    .padding([10, 20])
                    .style(iced::theme::Button::Primary)
                    .on_press(FormatSelectorMessage::Confirm),
            );

        // Main layout
        let content = column![
            title,
            text("Quick Select:").size(14),
            presets_row,
            text("Available Formats:").size(14),
            format_list,
            summary,
            actions,
        ]
        .spacing(16)
        .padding(20)
        .align_items(Alignment::Start);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Render the scrollable format list
    fn render_format_list(&self) -> Element<FormatSelectorMessage> {
        let formats: Vec<&VideoFormat> = if self.selected_preset == QualityPreset::AudioOnly {
            self.video.audio_formats()
        } else {
            self.video.combined_formats()
        };

        // If no combined formats (weird case for some videos), fallback to video only or all
        let formats = if formats.is_empty() && self.selected_preset != QualityPreset::AudioOnly {
            self.video.video_formats()
        } else {
            formats
        };

        let format_items: Vec<Element<FormatSelectorMessage>> = formats
            .iter()
            .map(|format| {
                let label = format.display_label();
                let is_selected = self.selected_format_id.as_deref() == Some(&format.format_id);

                let style = if is_selected {
                    iced::theme::Button::Primary
                } else {
                    iced::theme::Button::Secondary
                };

                button(text(label))
                    .width(Length::Fill)
                    .style(style)
                    .on_press(FormatSelectorMessage::FormatSelected(
                        format.format_id.clone(),
                    ))
                    .into()
            })
            .collect();

        let list = Column::with_children(format_items).spacing(8).padding(10);

        scrollable(list).height(Length::Fixed(250.0)).into()
    }
}
