use eframe::egui::{self, Align, ComboBox, Layout, RichText, Ui, vec2};

use crate::mixer::{AppStream, Config};

use super::{
    app_icons::Icons,
    mixer_channels::{Action, DragItem},
    preferences::Language,
    theme::MUTED,
};

pub(super) fn show(
    ui: &mut Ui,
    config: &Config,
    streams: &[AppStream],
    icons: &mut Icons,
    lang: Language,
) -> Option<Action> {
    ui.strong(lang.text("Застосунки", "Applications"));
    ui.add_space(4.0);
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
        ui.push_id(&stream.key, |ui| {
            ui.horizontal(|ui| {
                let route_width = 140.0_f32.min(ui.available_width() * 0.4);
                let identity_width =
                    ui.available_width() - route_width - ui.spacing().item_spacing.x;
                ui.dnd_drag_source(
                    ui.make_persistent_id("app-drag"),
                    DragItem::Application(stream.key.clone()),
                    |ui| {
                        ui.allocate_ui_with_layout(
                            vec2(identity_width, 36.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.set_min_size(vec2(identity_width, 36.0));
                                let (grip, _) =
                                    ui.allocate_exact_size(vec2(10.0, 24.0), egui::Sense::hover());
                                for x in [-2.0, 2.0] {
                                    for y in [-4.0, 0.0, 4.0] {
                                        ui.painter().circle_filled(
                                            grip.center() + vec2(x, y),
                                            0.8,
                                            MUTED,
                                        );
                                    }
                                }
                                icons.show(ui, stream);
                                ui.add(egui::Label::new(&stream.label).truncate())
                                    .on_hover_text(&stream.label);
                            },
                        );
                    },
                )
                .response
                .on_hover_text(lang.text("Перетягни на канал", "Drag onto a channel"));
                let mut selected = stream.channel;
                ComboBox::from_id_salt("app-route")
                    .width(route_width)
                    .selected_text(selected.map_or(lang.text("Авто", "Auto"), |channel| {
                        config.name(channel)
                    }))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected, None, lang.text("Авто", "Auto"));
                        for channel in config.channels() {
                            ui.selectable_value(&mut selected, Some(channel), config.name(channel));
                        }
                    });
                if selected != stream.channel {
                    action = Some(Action::Route(stream.key.clone(), selected));
                }
            });
        });
    }
    action
}

#[cfg(test)]
mod tests;
