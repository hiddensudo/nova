use eframe::egui::{self, Color32, Frame, RichText, Stroke, Ui, vec2};

use crate::mixer::{AppStream, Channel, Config};

use super::{
    app_icons::Icons,
    icons::{self, Icon},
    preferences::Language,
    theme::{BORDER, MUTED, card},
};

#[derive(Clone)]
pub(super) enum DragItem {
    Channel(Channel),
    Application(String),
}

pub(super) enum Action {
    Level(Channel, u8),
    Mute(Channel, bool),
    Color(Channel, u32),
    Route(String, Option<Channel>),
    Edit(Channel),
    Reorder(Channel, Channel),
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
    let order = config.channels();
    let width = ui.available_width();
    for (index, channel) in order.iter().copied().enumerate() {
        ui.push_id(channel, |ui| {
            let tint = color(config.color(channel));
            let frame = Frame::new()
                .inner_margin(egui::Margin::symmetric(9, 6))
                .corner_radius(8)
                .fill(super::theme::RAISED);
            let (row, application) = ui.dnd_drop_zone::<DragItem, _>(frame, |ui| {
                ui.set_width((width - 18.0).max(100.0));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.dnd_drag_source(
                        ui.make_persistent_id("channel-handle"),
                        DragItem::Channel(channel),
                        |ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(14.0, 26.0), egui::Sense::hover());
                            for x in [-2.5, 2.5] {
                                for y in [-5.0, 0.0, 5.0] {
                                    ui.painter().circle_filled(
                                        rect.center() + vec2(x, y),
                                        1.0,
                                        MUTED,
                                    );
                                }
                            }
                        },
                    )
                    .response
                    .on_hover_text(lang.text("Перетягни, щоб змінити порядок", "Drag to reorder"));
                    let name = config.name(channel);
                    let title = ui.add_sized(
                        [75.0, 26.0],
                        egui::Label::new(RichText::new(name).strong().color(tint)).truncate(),
                    );
                    title.on_hover_text(name);
                    let mut level = config.level(channel);
                    let width = (ui.available_width() - 143.0).max(24.0);
                    if super::controls::fader(ui, &mut level, 100, tint, width).changed() {
                        action = Some(Action::Level(channel, level));
                    }
                    ui.add_sized([39.0, 26.0], egui::Label::new(format!("{level}%")));
                    let muted = config.is_muted(channel);
                    if icons::button(
                        ui,
                        Icon::Speaker(muted),
                        if muted {
                            lang.text("Увімкнути звук", "Unmute")
                        } else {
                            lang.text("Вимкнути звук", "Mute")
                        },
                        muted,
                        false,
                    )
                    .clicked()
                    {
                        action = Some(Action::Mute(channel, !muted));
                    }
                    let active = !muted
                        && level > 0
                        && streams
                            .iter()
                            .any(|app| app.channel == Some(channel) && app.playing);
                    meter(ui, active, tint, lang);
                    ui.menu_button("...", |ui| {
                        if ui
                            .add_enabled(
                                index > 0,
                                egui::Button::new(lang.text("Перемістити вище", "Move up")),
                            )
                            .clicked()
                        {
                            action = Some(Action::Reorder(channel, order[index - 1]));
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                index + 1 < order.len(),
                                egui::Button::new(lang.text("Перемістити нижче", "Move down")),
                            )
                            .clicked()
                        {
                            action = Some(Action::Reorder(channel, order[index + 1]));
                            ui.close();
                        }
                        if matches!(channel, Channel::Custom(_))
                            && ui
                                .button(lang.text("Редагувати канал", "Edit channel"))
                                .clicked()
                        {
                            action = Some(Action::Edit(channel));
                            ui.close();
                        }
                        ui.separator();
                        let mut chosen = tint;
                        if ui.color_edit_button_srgba(&mut chosen).changed() {
                            action = Some(Action::Color(channel, hex(chosen)));
                        }
                    });
                });
            });
            if let Some(payload) = application {
                action = Some(match payload.as_ref() {
                    DragItem::Channel(moved) => Action::Reorder(*moved, channel),
                    DragItem::Application(key) => Action::Route(key.clone(), Some(channel)),
                });
            }
            if matches!(
                row.response.dnd_hover_payload::<DragItem>().as_deref(),
                Some(DragItem::Channel(_))
            ) {
                ui.painter().hline(
                    row.response.rect.x_range(),
                    row.response.rect.top(),
                    Stroke::new(2.0, super::theme::ACCENT),
                );
            }
        });
        ui.add_space(4.0);
    }
    action
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

pub(super) fn split(
    ui: &mut Ui,
    config: &Config,
    streams: &[AppStream],
    icons: &mut Icons,
    lang: Language,
) -> Option<Action> {
    let rows = card(ui, |ui| channels(ui, config, streams, lang)).inner;
    ui.add_space(8.0);
    card(ui, |ui| {
        super::applications::show(ui, config, streams, icons, lang)
    })
    .inner
    .or(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_a_channel_uses_the_reorder_payload_without_losing_it_as_an_app() {
        let context = egui::Context::default();
        super::super::theme::configure(&context);
        let config = Config::default();
        let mut action = None;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(900.0, 500.0),
                )),
                ..Default::default()
            },
            |ui| {
                action = channels(ui, &config, &[], Language::Ukrainian);
            },
        );
        output.textures_delta.clear();
        assert!(action.is_none());
        let text_position = |shape: &egui::Shape| -> Option<egui::Pos2> {
            if let egui::Shape::Text(text) = shape
                && text.galley.text() == "Chat"
            {
                return Some(text.visual_bounding_rect().center());
            }
            None
        };
        let target = output
            .shapes
            .iter()
            .find_map(|shape| text_position(&shape.shape))
            .expect("Chat row");
        egui::DragAndDrop::set_payload(&context, DragItem::Channel(Channel::Game));
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(900.0, 500.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(target),
                    egui::Event::PointerButton {
                        pos: target,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                action = channels(ui, &config, &[], Language::Ukrainian);
            },
        );
        output.textures_delta.clear();
        assert!(matches!(
            action,
            Some(Action::Reorder(Channel::Game, Channel::Chat))
        ));
    }
}
