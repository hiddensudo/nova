use eframe::egui::{self, Frame, Margin, RichText, Stroke, Ui};

use super::{
    engine::Engine,
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
                Frame::new()
                    .fill(CARD)
                    .corner_radius(6)
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("Мікрофон").color(ACCENT).strong());
                    });
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(28)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.microphone(ui);
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
            });
    }

    fn status(&self, ui: &mut Ui) {
        if let Some(level) = self.pending_level() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new(format!("Застосовую {}…", level.label())).color(MUTED));
            });
        } else if let Some(error) = &self.error {
            ui.label(RichText::new(error).color(WARNING));
        } else if let Some(level) = self.level {
            ui.label(RichText::new(format!("Поточний рівень · {}", level.label())).color(SUCCESS));
        } else {
            ui.label(RichText::new("Зчитую рівень із донгла…").color(MUTED));
        }
    }
}
