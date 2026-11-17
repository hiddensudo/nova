use std::{path::PathBuf, sync::mpsc, thread};

use eframe::egui::{self, Context, Ui};

use crate::device::eq::{self, Profile};

#[derive(Default)]
pub(super) struct EqPage {
    active: Option<Profile>,
    request: Option<mpsc::Receiver<Result<Profile, String>>>,
    error: Option<String>,
    path: String,
}

impl EqPage {
    fn run(
        &mut self,
        ctx: &Context,
        task: impl FnOnce() -> Result<Profile, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.request = Some(rx);
        self.error = None;
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(task());
            ctx.request_repaint();
        });
    }

    pub(super) fn poll(&mut self) {
        let Some(request) = &self.request else { return };
        let result = match request.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("Запит до EQ перервався.".into()),
        };
        self.request = None;
        match result {
            Ok(profile) => self.active = Some(profile),
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn show(&mut self, ui: &mut Ui, device_busy: bool) {
        ui.heading("Апаратний EQ");
        if self.active.is_none() && self.error.is_none() && self.request.is_none() && !device_busy {
            self.run(ui.ctx(), eq::current);
        }
        if let Some(profile) = &self.active {
            ui.label(format!("Поточний профіль · {}", profile.name));
        }
        if let Some(error) = &self.error {
            ui.colored_label(super::theme::WARNING, error);
        }
        if self.busy() || device_busy {
            ui.spinner();
        }

        let enabled = !self.busy() && !device_busy;
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal_wrapped(|ui| {
                for name in eq::PRESETS {
                    if ui.button(name).clicked() {
                        self.run(ui.ctx(), move || eq::apply(&Profile::preset(name)?));
                    }
                }
                if ui.button("Зчитати профіль").clicked() {
                    self.run(ui.ctx(), eq::current);
                }
            });
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.path).hint_text("Шлях до JSON-профілю"),
                );
                if ui
                    .add_enabled(
                        !self.path.trim().is_empty(),
                        egui::Button::new("Імпортувати"),
                    )
                    .clicked()
                {
                    let path = PathBuf::from(self.path.trim());
                    self.run(ui.ctx(), move || eq::apply(&eq::from_file(&path)?));
                }
            });
        });
    }

    pub(super) fn busy(&self) -> bool {
        self.request.is_some()
    }
}
