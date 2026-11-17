use eframe::egui::{self, Button, Frame, Margin, Popup, PopupCloseBehavior, RichText, Ui};

use crate::device::Operation;

use super::{
    engine::{Engine, Page},
    icons::{self, Icon},
    preferences::Language,
    theme::{ACCENT, BACKGROUND, BORDER, MUTED, SIDEBAR, card},
};

impl Page {
    const NAVIGATION: [(Self, Icon); 5] = [
        (Self::Overview, Icon::Overview),
        (Self::Microphone, Icon::Mic),
        (Self::Eq, Icon::Eq),
        (Self::Mixer, Icon::Mixer),
        (Self::Settings, Icon::Headset),
    ];

    pub fn name(self, language: Language) -> &'static str {
        match self {
            Self::Overview => language.text("Огляд", "Overview"),
            Self::Microphone => language.text("Мікрофон", "Microphone"),
            Self::Eq => "EQ",
            Self::Mixer => language.text("Мікшер", "Mixer"),
            Self::Settings => language.text("Пристрій", "Device"),
        }
    }
}

impl Engine {
    pub(super) fn draw(&mut self, ui: &mut Ui) {
        let lang = self.prefs.language;
        egui::Panel::left("nova-rail")
            .resizable(false)
            .show_separator_line(true)
            .exact_size(58.0)
            .frame(
                Frame::new()
                    .fill(SIDEBAR)
                    .inner_margin(Margin::symmetric(6, 10)),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if icons::button(ui, Icon::Logo, "Nova Control", false, false).clicked() {
                        self.page = Page::Overview;
                    }
                    ui.add_space(11.0);
                    for (page, icon) in Page::NAVIGATION {
                        if icons::button(ui, icon, page.name(lang), self.page == page, true)
                            .clicked()
                        {
                            self.page = page;
                        }
                        ui.add_space(2.0);
                    }
                });
            });

        egui::Panel::top("nova-status-bar")
            .resizable(false)
            .show_separator_line(true)
            .exact_size(54.0)
            .frame(
                Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(Margin::symmetric(17, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.strong("Nova Control");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let quick = icons::button(
                            ui,
                            Icon::Mic,
                            lang.text("Швидке налаштування Sidetone", "Quick Sidetone"),
                            false,
                            false,
                        );
                        Popup::from_toggle_button_response(&quick)
                            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                            .width(280.0)
                            .show(|ui| {
                                ui.set_min_width(260.0);
                                ui.set_max_width(280.0);
                                self.sidetone_card(ui, lang);
                            });
                        self.status_battery(ui);
                        self.status_connection(ui, true);
                    });
                });
            });

        let page = self.page;
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(Margin::symmetric(18, 15)),
            )
            .show(ui, |ui| {
                ui.style_mut()
                    .visuals
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .color = BORDER;
                egui::ScrollArea::vertical()
                    .id_salt(page as u8)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(RichText::new(page.name(lang)).size(25.0).strong());
                        ui.add_space(7.0);
                        let mic_busy = self.mic_busy();
                        match page {
                            Page::Overview => self.overview(ui, lang),
                            Page::Microphone => self.microphone(ui, lang),
                            Page::Mixer => self.mixer.show(ui, lang),
                            Page::Settings => self.settings.show(
                                ui,
                                &mut self.prefs,
                                mic_busy || self.eq.busy(),
                                self.desktop.as_mut(),
                            ),
                            Page::Eq => {
                                self.eq
                                    .show(ui, &mut self.prefs, mic_busy || self.settings.busy())
                            }
                        }
                    });
            });
    }

    fn microphone(&mut self, ui: &mut Ui, lang: Language) {
        let device_busy = self.mic_busy() || self.eq.busy();
        if ui.available_width() >= 540.0 {
            ui.columns(2, |columns| {
                card(&mut columns[0], |ui| self.sidetone_card(ui, lang));
                card(&mut columns[1], |ui| {
                    self.settings.gain(ui, lang, device_busy)
                });
            });
        } else {
            card(ui, |ui| self.sidetone_card(ui, lang));
            card(ui, |ui| self.settings.gain(ui, lang, device_busy));
        }
    }

    fn sidetone_card(&mut self, ui: &mut Ui, lang: Language) {
        let title = ui.strong("Sidetone");
        self.sidetone_slider(ui).labelled_by(title.id);
        if let Some(level) = self.pending_level() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(if self.saving() {
                    lang.text("Зберігаю в гарнітурі…", "Saving to headset…")
                        .to_owned()
                } else {
                    format!("{} {}…", lang.text("Застосовую", "Applying"), level.label())
                });
            });
        } else if let Some(error) = &self.error {
            ui.colored_label(super::theme::WARNING, error);
        } else if self.can_save() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(lang.text("Застосовано", "Applied")).color(MUTED));
                let save = Button::new(
                    RichText::new(lang.text("Зберегти в гарнітурі", "Save to headset"))
                        .color(SIDEBAR),
                )
                .fill(ACCENT);
                if ui
                    .add_enabled(!self.eq.busy() && !self.settings.busy(), save)
                    .clicked()
                {
                    self.submit(ui.ctx(), Operation::Save);
                }
            });
        } else if self.level.is_none() {
            ui.label(
                RichText::new(lang.text("Зчитую рівень із донгла…", "Reading sidetone…"))
                    .color(MUTED),
            );
        }
    }
}
