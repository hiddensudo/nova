use eframe::egui::{self, Color32, ComboBox, Ui};

use crate::mixer::{Channel, ChannelIcon, Config, default_color};

use super::{
    mixer_channels::{color, hex},
    preferences::Language,
    theme::WARNING,
};

pub(super) struct ChannelDialog {
    pub channel: Option<Channel>,
    pub name: String,
    pub tint: Color32,
    pub icon: ChannelIcon,
    pub position: usize,
    pub error: Option<String>,
}

pub(super) enum Interaction {
    Save,
    Delete,
    Cancel,
}

impl ChannelDialog {
    pub fn new(channel: Option<Channel>, config: &Config) -> Self {
        let channels = config.channels();
        Self {
            channel,
            name: channel.map_or_else(String::new, |id| config.name(id).into()),
            tint: color(channel.map_or_else(default_color, |id| config.color(id))),
            icon: channel.map_or(ChannelIcon::Globe, |id| config.icon(id)),
            position: channel
                .and_then(|id| channels.iter().position(|&row| row == id))
                .unwrap_or(channels.len()),
            error: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, config: &Config, lang: Language) -> Option<Interaction> {
        let mut action = None;
        let title = if self.channel.is_none() {
            lang.text("Новий канал", "New channel")
        } else {
            lang.text("Редагувати канал", "Edit channel")
        };
        let response =
            egui::Modal::new(egui::Id::unique("nova-channel-dialog")).show(ui.ctx(), |ui| {
                ui.set_min_width(285.0);
                ui.heading(title);
                ui.label(lang.text("Назва", "Name"));
                ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(24));
                ui.horizontal(|ui| {
                    ui.label(lang.text("Колір", "Color"));
                    ui.color_edit_button_srgba(&mut self.tint);
                    ComboBox::from_id_salt("icon")
                        .selected_text(icon_label(self.icon))
                        .show_ui(ui, |ui| {
                            for icon in [
                                ChannelIcon::Game,
                                ChannelIcon::Music,
                                ChannelIcon::Chat,
                                ChannelIcon::Globe,
                                ChannelIcon::Stream,
                            ] {
                                ui.selectable_value(&mut self.icon, icon, icon_label(icon));
                            }
                        });
                });
                let channels: Vec<_> = config
                    .channels()
                    .into_iter()
                    .filter(|&id| Some(id) != self.channel)
                    .collect();
                ui.horizontal(|ui| {
                    ui.label(lang.text("Порядок", "Order"));
                    let label = |position| {
                        channels.get(position).map_or_else(
                            || lang.text("У кінці", "At the end").into(),
                            |&channel| {
                                format!("{} {}", lang.text("Перед", "Before"), config.name(channel))
                            },
                        )
                    };
                    ComboBox::from_id_salt("position")
                        .selected_text(label(self.position))
                        .show_ui(ui, |ui| {
                            for position in 0..=channels.len() {
                                ui.selectable_value(&mut self.position, position, label(position));
                            }
                        });
                });
                if let Some(error) = &self.error {
                    ui.colored_label(WARNING, error);
                }
                ui.horizontal_wrapped(|ui| {
                    if self.channel.is_some()
                        && ui
                            .button(lang.text("Видалити канал", "Delete channel"))
                            .clicked()
                    {
                        action = Some(Interaction::Delete);
                    }
                    if ui.button(lang.text("Скасувати", "Cancel")).clicked() {
                        action = Some(Interaction::Cancel);
                    }
                    if ui
                        .add_enabled(
                            !self.name.trim().is_empty(),
                            egui::Button::new(lang.text("Зберегти", "Save")),
                        )
                        .clicked()
                    {
                        action = Some(Interaction::Save);
                    }
                });
            });
        if response.should_close() {
            action = Some(Interaction::Cancel);
        }
        action
    }

    pub fn apply(&self, config: &mut Config, delete: bool) -> Result<(), String> {
        match self.channel {
            Some(channel) if delete => config.remove_channel(channel),
            Some(channel) => config.edit_channel(
                channel,
                &self.name,
                hex(self.tint),
                self.icon,
                self.position,
            ),
            None => config
                .add_channel(&self.name, hex(self.tint), self.icon, self.position)
                .map(|_| ()),
        }
    }
}

fn icon_label(icon: ChannelIcon) -> &'static str {
    match icon {
        ChannelIcon::Game => "Game",
        ChannelIcon::Music => "Music",
        ChannelIcon::Chat => "Chat",
        ChannelIcon::Globe => "Browser",
        ChannelIcon::Stream => "Stream",
    }
}
