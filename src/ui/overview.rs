use eframe::egui::{self, RichText, Ui};

use super::{
    engine::{Engine, Page},
    icons::{self, Icon},
    preferences::Language,
    theme::{ACCENT, MUTED, SUCCESS, WARNING, card},
};

impl Engine {
    pub fn overview(&mut self, ui: &mut Ui, lang: Language) {
        ui.columns(2, |columns| {
            card(&mut columns[0], |ui| {
                ui.horizontal(|ui| {
                    icons::label(ui, Icon::Headset, ACCENT);
                    ui.label(RichText::new("SteelSeries").color(MUTED));
                });
                ui.add_space(5.0);
                ui.label(RichText::new("Arctis Nova 7 Gen 2").size(21.0).strong());
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    self.status_connection(ui, false);
                    self.status_battery(ui);
                });
            });
            columns[0].add_space(7.0);
            card(&mut columns[0], |ui| {
                ui.horizontal(|ui| {
                    ui.strong(lang.text("Канали", "Channels"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("›")
                            .on_hover_text(Page::Mixer.name(lang))
                            .clicked()
                        {
                            self.page = Page::Mixer;
                        }
                    });
                });
                self.mixer.summary(ui, lang);
            });
            card(&mut columns[1], |ui| {
                ui.strong(lang.text("Швидкі дії", "Quick actions"));
                let sidetone = self.level.map_or("—", |level| level.label());
                let preset = self.eq.name().unwrap_or("—");
                for (page, label, detail) in [
                    (Page::Microphone, "Sidetone", sidetone),
                    (Page::Eq, "EQ", preset),
                    (Page::Mixer, Page::Mixer.name(lang), "ChatMix"),
                ] {
                    if ui
                        .add_sized(
                            [ui.available_width(), 30.0],
                            egui::Button::new(format!("{label}  ·  {detail}  ›")),
                        )
                        .clicked()
                    {
                        self.page = page;
                    }
                }
            });
        });
    }

    pub fn status_connection(&self, ui: &mut Ui, header: bool) {
        let lang = self.prefs.language;
        let (label, color) = match self.settings.data() {
            Some(state) if state.connected => (
                if header {
                    lang.text("Підключено", "Connected")
                } else {
                    "2,4 GHz"
                },
                SUCCESS,
            ),
            Some(_) => (lang.text("Не підключено", "Disconnected"), WARNING),
            None if self.settings.failed() => {
                (lang.text("Немає зв’язку", "Not connected"), WARNING)
            }
            None => (lang.text("З’єднання…", "Connecting…"), MUTED),
        };
        let dot = |ui: &mut Ui| {
            ui.colored_label(color, "●");
        };
        let text = |ui: &mut Ui| {
            ui.label(RichText::new(label).small().color(MUTED));
        };
        ui.horizontal(|ui| {
            if ui.layout().prefer_right_to_left() {
                text(ui);
                dot(ui);
            } else {
                dot(ui);
                text(ui);
            }
        });
    }

    pub fn status_battery(&self, ui: &mut Ui) {
        let Some(state) = self.settings.data() else {
            return;
        };
        let Some(percent) = state.battery else { return };
        let charging = state.charging.is_charging();
        let color = if percent <= 20 && !charging {
            WARNING
        } else {
            MUTED
        };
        ui.horizontal(|ui| {
            let icon = |ui: &mut Ui| icons::label(ui, Icon::Battery(charging), color);
            let text = |ui: &mut Ui| {
                ui.label(RichText::new(format!("{percent}%")).small().color(color));
            };
            if ui.layout().prefer_right_to_left() {
                text(ui);
                icon(ui);
            } else {
                icon(ui);
                text(ui);
            }
        });
    }
}
