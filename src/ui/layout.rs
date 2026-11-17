use eframe::egui::{self, Button, Frame, Margin, RichText, Stroke, Ui};

use crate::device::Operation;

use super::{
    engine::{Engine, Page},
    theme::{ACCENT, BACKGROUND, BORDER, CARD, MUTED, SIDEBAR, SUCCESS, WARNING},
};

impl Engine {
    pub(super) fn draw(&mut self, ui: &mut Ui) {
        egui::Panel::left("engine")
            .resizable(false)
            .show_separator_line(false)
            .exact_size(172.0)
            .frame(Frame::new().fill(SIDEBAR).inner_margin(Margin::same(20)))
            .show(ui, |ui| {
                ui.label(RichText::new("Nova Control").size(18.0).strong());
                ui.add_space(32.0);
                ui.label(RichText::new("ENGINE").size(11.0).strong().color(MUTED));
                ui.add_space(7.0);
                for (page, label) in [
                    (Page::Microphone, "Мікрофон"),
                    (Page::Eq, "EQ"),
                    (Page::Mixer, "Мікшер"),
                    (Page::Settings, "Пристрій"),
                ] {
                    let text = RichText::new(label).strong().color(if self.page == page {
                        ACCENT
                    } else {
                        MUTED
                    });
                    if ui
                        .add_sized(
                            [ui.available_width(), 30.0],
                            Button::new(text).selected(self.page == page),
                        )
                        .clicked()
                    {
                        self.page = page;
                    }
                }
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(28)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        Page::Microphone => self.microphone(ui),
                        Page::Mixer => self.mixer.show(ui),
                        Page::Settings => {
                            let busy = self.mic_busy() || self.eq.busy();
                            self.settings.show(ui, busy);
                        }
                        Page::Eq => {
                            let busy = self.mic_busy() || self.settings.busy();
                            self.eq.show(ui, busy);
                        }
                    });
            });
    }

    fn microphone(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("SteelSeries").size(13.0).color(MUTED));
        ui.label(RichText::new("Arctis Nova 7 Gen 2").size(28.0).strong());
        ui.add_space(20.0);

        Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(10)
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                let heading = ui.label(RichText::new("Mic Sidetone").size(18.0).strong());
                ui.add_space(5.0);
                ui.label(RichText::new("Рівень власного голосу в навушниках.").color(MUTED));
                ui.add_space(16.0);
                self.sidetone_slider(ui).labelled_by(heading.id);
                ui.add_space(16.0);
                self.status(ui);
                ui.add_space(10.0);
                if ui
                    .add_enabled(self.can_save(), Button::new("Зберегти в гарнітурі"))
                    .on_hover_text("Записати поточні налаштування в пам’ять Nova 7 Gen 2.")
                    .clicked()
                {
                    self.submit(ui.ctx(), Operation::Save);
                }
            });
    }

    fn status(&self, ui: &mut Ui) {
        if let Some(level) = self.pending_level() {
            ui.horizontal(|ui| {
                ui.spinner();
                let message = if self.saving() {
                    "Зберігаю в гарнітурі…".into()
                } else {
                    format!("Застосовую {}…", level.label())
                };
                ui.label(RichText::new(message).color(MUTED));
            });
        } else if let Some(error) = &self.error {
            ui.label(RichText::new(error).color(WARNING));
        } else if let Some(level) = self.level {
            let message = if self.saved_level == Some(level) {
                "Команду збереження прийнято"
            } else {
                "Поточний рівень"
            };
            ui.label(RichText::new(format!("{message} · {}", level.label())).color(SUCCESS));
        } else {
            ui.label(RichText::new("Зчитую рівень із донгла…").color(MUTED));
        }
    }
}
