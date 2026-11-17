use eframe::egui::{self, RichText, Ui};

use super::{
    engine::{Engine, Page},
    icons::{self, Icon},
    preferences::Language,
    theme::{ACCENT, MUTED, SUCCESS, WARNING, card},
};

impl Engine {
    pub fn overview(&mut self, ui: &mut Ui, lang: Language) {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                icons::label(ui, Icon::Headset, ACCENT);
                ui.label(RichText::new("SteelSeries").color(MUTED));
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Arctis Nova 7 Gen 2").size(26.0).strong());
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                self.status_connection(ui, false);
                self.status_battery(ui);
                if let Some(state) = self.settings.data() {
                    ui.separator();
                    ui.label(if state.charging.is_charging() {
                        lang.text("Заряджається", "Charging")
                    } else {
                        lang.text("Живлення від батареї", "On battery")
                    });
                }
            });
        });
        ui.add_space(10.0);
        ui.columns(2, |columns| {
            let channels = card(&mut columns[1], |ui| {
                ui.strong(lang.text("Канали", "Channels"));
                ui.add_space(6.0);
                self.mixer.summary(ui, lang);
                ui.add_space(5.0);
                if ui
                    .button(lang.text("Керувати каналами", "Manage channels"))
                    .clicked()
                {
                    self.navigate(Page::Mixer);
                }
                ui.min_rect().height()
            });
            let height = channels.inner;
            card(&mut columns[0], |ui| {
                ui.set_min_height(height);
                ui.strong(lang.text("Аудіо", "Audio"));
                ui.add_space(4.0);
                let preset = self.eq.name().unwrap_or("—").to_owned();
                ui.label(format!("EQ · {preset}"));
                ui.label(format!(
                    "Sidetone · {}",
                    self.level.map_or("—", |level| level.label())
                ));
                ui.add_space(8.0);
                for (page, label) in [
                    (Page::Eq, lang.text("Відкрити EQ", "Open EQ")),
                    (
                        Page::Mixer,
                        lang.text("Мікшер і мікрофон", "Mixer and microphone"),
                    ),
                ] {
                    if ui
                        .add_sized([ui.available_width(), 32.0], egui::Button::new(label))
                        .clicked()
                    {
                        self.navigate(page);
                    }
                }
            });
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            ui.strong(lang.text("Гарнітура", "Headset"));
            if let Some(state) = self.settings.data() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "{} · {}/7",
                        lang.text("Мікрофон", "Microphone"),
                        state.mic_volume
                    ));
                    ui.separator();
                    ui.label(format!(
                        "Mute LED · {}",
                        ["Off", "Low", "Medium", "High"][usize::from(state.mute_led)]
                    ));
                    ui.separator();
                    ui.label(format!(
                        "{} · {} {}",
                        lang.text("Автовимкнення", "Auto-off"),
                        state.sleep_minutes,
                        lang.text("хв", "min")
                    ));
                });
            }
            if ui
                .button(lang.text("Налаштування пристрою", "Device settings"))
                .clicked()
            {
                self.navigate(Page::Settings);
            }
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
            let icon = |ui: &mut Ui| super::battery::icon(ui, percent, charging, color);
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
