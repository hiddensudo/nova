use eframe::egui::{self, Color32, ComboBox, Frame, Response, RichText, Slider, Stroke, Ui, vec2};

use crate::mixer::{AppStream, Channel, Config};

use super::{
    icons::{self, Icon},
    preferences::Language,
    theme::{BORDER, MUTED, card},
};

pub(super) enum Action {
    Level(Channel, u8),
    Mute(Channel, bool),
    Color(Channel, u32),
    Route(String, Option<Channel>),
    Edit(Channel),
}

pub(super) fn color(rgb: u32) -> Color32 {
    let [_, red, green, blue] = rgb.to_be_bytes();
    Color32::from_rgb(red, green, blue)
}

pub(super) fn hex(tint: Color32) -> u32 {
    u32::from_be_bytes([0, tint.r(), tint.g(), tint.b()])
}

pub(super) fn channels(
    ui: &mut Ui,
    config: &Config,
    streams: &[AppStream],
    lang: Language,
) -> Option<Action> {
    let mut action = None;
    for channel in config.channels() {
        ui.push_id(channel, |ui| {
            let (row, drop) = ui.dnd_drop_zone::<String, _>(Frame::NONE, |ui| {
                let base_level = config.level(channel);
                let mut level = base_level;
                let mut tint = color(config.color(channel));
                let muted = config.is_muted(channel);
                ui.horizontal(|ui| {
                    icons::label(ui, Icon::Channel(config.icon(channel)), tint);
                    let name = config.name(channel);
                    let label = ui.add_sized(
                        [74.0, 22.0],
                        egui::Label::new(RichText::new(name).strong()).truncate(),
                    );
                    label.on_hover_text(name);
                    let available = ui.available_width();
                    if fader(ui, &mut level, tint, (available - 108.0).max(26.0)).changed()
                        && level != base_level
                    {
                        action = Some(Action::Level(channel, level));
                    }
                    let volume = format!("{level}%");
                    ui.add_sized(
                        [33.0, 18.0],
                        egui::Label::new(RichText::new(volume).monospace().small()).truncate(),
                    );
                    let mute = if muted {
                        lang.text("Увімкнути звук", "Unmute")
                    } else {
                        lang.text("Вимкнути звук", "Mute")
                    };
                    let icon = Icon::Speaker(muted);
                    if icons::button(ui, icon, mute, muted, false).clicked() {
                        action = Some(Action::Mute(channel, !muted));
                    }
                    let active = !muted
                        && level > 0
                        && streams
                            .iter()
                            .any(|app| app.channel == Some(channel) && app.playing);
                    meter(ui, active, tint, lang);
                    if matches!(channel, Channel::Custom(_))
                        && ui
                            .small_button("⋯")
                            .on_hover_text(lang.text("Редагувати канал", "Edit channel"))
                            .clicked()
                    {
                        action = Some(Action::Edit(channel));
                    } else if channel.builtin_index().is_some()
                        && ui.color_edit_button_srgba(&mut tint).changed()
                    {
                        action = Some(Action::Color(channel, hex(tint)));
                    }
                });
            });
            row.response.on_hover_text(lang.text(
                "Сюди можна перетягнути застосунок",
                "Drop an application here",
            ));
            if let Some(key) = drop {
                action = Some(Action::Route((*key).clone(), Some(channel)));
            }
        });
        ui.separator();
    }
    action
}

fn fader(ui: &mut Ui, level: &mut u8, tint: Color32, width: f32) -> Response {
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = width;
        ui.visuals_mut().selection.bg_fill = tint;
        ui.add(
            Slider::new(level, 0..=100)
                .show_value(false)
                .trailing_fill(true),
        )
    })
    .inner
}

fn meter(ui: &mut Ui, active: bool, tint: Color32, lang: Language) {
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 17.0), egui::Sense::hover());
    for (index, height) in [4.0, 7.0, 11.0, 15.0].into_iter().enumerate() {
        let x = rect.left() + index as f32 * 4.0 + 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(x, rect.bottom() - 2.0),
                egui::pos2(x, rect.bottom() - 2.0 - height),
            ],
            Stroke::new(2.0, if active { tint } else { BORDER }),
        );
    }
    response.on_hover_text(if active {
        lang.text("Активний аудіопотік", "Audio stream active")
    } else {
        lang.text("Немає активного потоку", "No active audio stream")
    });
}

pub(super) fn applications(
    ui: &mut Ui,
    config: &Config,
    streams: &[AppStream],
    lang: Language,
) -> Option<Action> {
    ui.strong(lang.text("Застосунки", "Applications"));
    if streams.is_empty() {
        ui.label(
            RichText::new(lang.text(
                "Запусти звук у застосунку, щоб призначити канал.",
                "Play audio in an application to assign its channel.",
            ))
            .color(MUTED),
        );
    }
    let mut action = None;
    for stream in streams {
        let mut selected = stream.channel;
        ui.horizontal_wrapped(|ui| {
            ui.dnd_drag_source(
                ui.make_persistent_id(("app-drag", &stream.key)),
                stream.key.clone(),
                |ui| {
                    ui.label(format!("⠿  {}", stream.label));
                },
            )
            .response
            .on_hover_text(lang.text("Перетягни на канал", "Drag to a channel"));
            ComboBox::from_id_salt(&stream.key)
                .width(77.0)
                .selected_text(selected.map_or(lang.text("Авто", "Auto"), |channel| {
                    config.name(channel)
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, None, lang.text("Авто", "Auto"));
                    for channel in config.channels() {
                        ui.selectable_value(&mut selected, Some(channel), config.name(channel));
                    }
                });
        });
        if selected != stream.channel {
            action = Some(Action::Route(stream.key.clone(), selected));
        }
    }
    action
}

pub(super) fn split(
    ui: &mut Ui,
    config: &Config,
    streams: &[AppStream],
    lang: Language,
) -> Option<Action> {
    if ui.available_width() < 710.0 {
        let rows = card(ui, |ui| channels(ui, config, streams, lang)).inner;
        ui.add_space(5.0);
        return card(ui, |ui| applications(ui, config, streams, lang))
            .inner
            .or(rows);
    }
    ui.horizontal_top(|ui| {
        let width = ui.available_width() - ui.spacing().item_spacing.x;
        let make_layout = || egui::Layout::top_down(egui::Align::Min);
        let rows = ui
            .allocate_ui_with_layout(vec2(width * 0.63, 0.0), make_layout(), |ui| {
                card(ui, |ui| channels(ui, config, streams, lang)).inner
            })
            .inner;
        let apps = ui
            .allocate_ui_with_layout(vec2(width * 0.37, 0.0), make_layout(), |ui| {
                card(ui, |ui| applications(ui, config, streams, lang)).inner
            })
            .inner;
        apps.or(rows)
    })
    .inner
}
