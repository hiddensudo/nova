use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{ComboBox, Context, Ui};

use crate::device::settings::{self, Change, Settings};

#[derive(Default)]
pub(super) struct SettingsPage {
    state: Option<Settings>,
    request: Option<mpsc::Receiver<Result<Settings, String>>>,
    error: Option<String>,
    next_read: Option<Instant>,
}

impl SettingsPage {
    fn run(
        &mut self,
        ctx: &Context,
        task: impl FnOnce() -> Result<Settings, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.request = Some(rx);
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
            Err(mpsc::TryRecvError::Disconnected) => Err("Запит налаштувань перервався.".into()),
        };
        self.next_read = Some(Instant::now() + Duration::from_secs(5));
        self.request = None;
        match result {
            Ok(state) => {
                self.state = Some(state);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn busy(&self) -> bool {
        self.request.is_some()
    }

    pub(super) fn show(&mut self, ui: &mut Ui, device_busy: bool) {
        ui.heading("Пристрій");
        if let Some(error) = &self.error {
            ui.colored_label(super::theme::WARNING, error);
        }
        if let Some(state) = self.state {
            ui.label(state.battery.map_or_else(
                || "Батарея · —".to_owned(),
                |percent| format!("Батарея · {percent}%"),
            ));
            ui.label(state.charging.label());
            let enabled = !self.busy() && !device_busy && state.connected && self.error.is_none();
            ui.add_enabled_ui(enabled, |ui| {
                let mic = select(
                    ui,
                    "Гучність мікрофона",
                    state.mic_volume,
                    0..=7,
                    |value| value.to_string(),
                );
                let sleep = select(
                    ui,
                    "Автовимкнення",
                    state.sleep_minutes,
                    [0, 1, 5, 10, 15, 30, 45, 60, 75, 90],
                    |value| {
                        if value == 0 {
                            "Вимкнено".into()
                        } else {
                            format!("{value} хв")
                        }
                    },
                );
                let led = select(
                    ui,
                    "Яскравість LED вимкненого мікрофона",
                    state.mute_led,
                    0..=3,
                    |value| value.to_string(),
                );
                let change = mic
                    .map(Change::MicVolume)
                    .or_else(|| sleep.map(Change::SleepMinutes))
                    .or_else(|| led.map(Change::MuteLed));
                if let Some(change) = change {
                    self.run(ui.ctx(), move || settings::apply(change));
                } else if ui.button("Зберегти в гарнітурі").clicked() {
                    self.run(ui.ctx(), settings::save);
                }
            });
        }
        if self.busy() {
            ui.spinner();
        } else if !device_busy {
            let now = Instant::now();
            match self.next_read {
                Some(deadline) if deadline > now => ui.ctx().request_repaint_after(deadline - now),
                _ => self.run(ui.ctx(), settings::current),
            }
        }
    }
}

fn select(
    ui: &mut Ui,
    title: &str,
    value: u8,
    options: impl IntoIterator<Item = u8>,
    label: impl Fn(u8) -> String,
) -> Option<u8> {
    let mut selected = value;
    ui.horizontal_wrapped(|ui| {
        ui.label(title);
        ComboBox::from_id_salt(title)
            .selected_text(label(value))
            .show_ui(ui, |ui| {
                for candidate in options {
                    ui.selectable_value(&mut selected, candidate, label(candidate));
                }
            });
    });
    (selected != value).then_some(selected)
}
