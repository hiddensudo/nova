use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{self, ComboBox, Context, Ui};

use crate::device::{
    self,
    settings::{self, Change, Settings},
};

use super::{
    notifications::Notifications,
    preferences::{Language, Preferences},
    theme::{WARNING, card},
};

#[derive(Default)]
pub(super) struct SettingsPage {
    state: Option<Settings>,
    request: Option<mpsc::Receiver<Result<Settings, String>>>,
    error: Option<String>,
    next_read: Option<Instant>,
    notices: Notifications,
    gain_drag: Option<u8>,
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

    pub fn poll(&mut self, prefs: &Preferences) {
        let Some(request) = &self.request else { return };
        let result = match request.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Headset settings request interrupted.".into())
            }
        };
        self.next_read = Some(Instant::now() + Duration::from_secs(5));
        self.request = None;
        match result {
            Ok(state) => {
                self.notices.observe(state, prefs);
                self.state = Some(state);
                self.error = None;
            }
            Err(error) => {
                let disconnected = !device::dongle_present();
                if self.error.as_ref() != Some(&error) {
                    self.notices.error(disconnected, &error, prefs);
                }
                if disconnected {
                    self.state = None;
                }
                self.error = Some(error);
            }
        }
    }

    pub fn busy(&self) -> bool {
        self.request.is_some()
    }

    pub fn data(&self) -> Option<Settings> {
        self.state
    }

    pub fn failed(&self) -> bool {
        self.error.is_some()
    }

    pub fn refresh(&mut self, ctx: &Context) {
        if self.busy() {
            return;
        }
        let now = Instant::now();
        match self.next_read {
            Some(deadline) if deadline > now => ctx.request_repaint_after(deadline - now),
            _ => self.run(ctx, settings::current),
        }
    }

    pub fn gain(&mut self, ui: &mut Ui, lang: Language, device_busy: bool) {
        let title = lang.text("Підсилення мікрофона", "Mic gain");
        ui.strong(title);
        let mut value = self
            .gain_drag
            .or(self.state.map(|state| state.mic_volume))
            .unwrap_or(0);
        let response = ui.add_enabled(
            self.enabled(device_busy),
            egui::Slider::new(&mut value, 0..=7)
                .text("/ 7")
                .trailing_fill(true),
        );
        if response.dragged() {
            self.gain_drag = Some(value);
        } else if response.drag_stopped() || response.changed() {
            self.gain_drag = None;
            let change = Change::MicVolume(value);
            self.run(ui.ctx(), move || settings::apply(change));
        }
        self.status(ui);
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        prefs: &mut Preferences,
        device_busy: bool,
        desktop: Option<&mut super::desktop::Desktop>,
    ) {
        let lang = prefs.language;
        if ui.available_width() >= 560.0 {
            ui.columns(2, |columns| {
                card(&mut columns[0], |ui| self.hardware(ui, lang, device_busy));
                card(&mut columns[1], |ui| Self::notifications(ui, prefs));
                columns[0].add_space(9.0);
                card(&mut columns[0], |ui| Self::application(ui, prefs, desktop));
            });
        } else {
            card(ui, |ui| self.hardware(ui, lang, device_busy));
            card(ui, |ui| Self::notifications(ui, prefs));
            card(ui, |ui| Self::application(ui, prefs, desktop));
        }
    }

    fn enabled(&self, device_busy: bool) -> bool {
        !self.busy()
            && !device_busy
            && self.state.is_some_and(|state| state.connected)
            && self.error.is_none()
    }

    fn hardware(&mut self, ui: &mut Ui, lang: Language, device_busy: bool) {
        ui.strong(lang.text("Гарнітура", "Headset"));
        let enabled = self.enabled(device_busy);
        if let Some(state) = self.state {
            ui.add_enabled_ui(enabled, |ui| {
                let sleep = select(
                    ui,
                    lang.text("Автовимкнення", "Auto-off"),
                    state.sleep_minutes,
                    [0, 1, 5, 10, 15, 30, 45, 60, 75, 90],
                    |value| {
                        if value == 0 {
                            "Off".into()
                        } else {
                            format!("{value} {}", lang.text("хв", "min"))
                        }
                    },
                );
                let led = select(ui, "Mute LED", state.mute_led, 0..=3, |value| {
                    ["Off", "Low", "Medium", "High"][usize::from(value)].into()
                });
                if let Some(change) = sleep
                    .map(Change::SleepMinutes)
                    .or_else(|| led.map(Change::MuteLed))
                {
                    self.run(ui.ctx(), move || settings::apply(change));
                } else if ui
                    .button(lang.text("Зберегти в гарнітурі", "Save to headset"))
                    .clicked()
                {
                    self.run(ui.ctx(), settings::save);
                }
            });
        }
        self.status(ui);
    }

    fn status(&self, ui: &mut Ui) {
        if let Some(error) = &self.error {
            ui.colored_label(WARNING, error);
        } else if self.busy() {
            ui.spinner();
        }
    }

    fn notifications(ui: &mut Ui, prefs: &mut Preferences) {
        let lang = prefs.language;
        ui.strong(lang.text("Системні сповіщення", "Desktop notifications"));
        let mut changed = ui
            .checkbox(
                &mut prefs.low_battery,
                lang.text("Низький заряд", "Low battery"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut prefs.disconnected,
                lang.text("Від’єднання гарнітури", "Headset disconnected"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut prefs.errors,
                lang.text("Помилки пристрою", "Device errors"),
            )
            .changed();
        Self::save_prefs(ui, prefs, changed);
    }

    fn application(
        ui: &mut Ui,
        prefs: &mut Preferences,
        desktop: Option<&mut super::desktop::Desktop>,
    ) {
        let before = prefs.language;
        ui.strong("Nova Control");
        ui.horizontal_wrapped(|ui| {
            ui.label(before.text("Мова", "Language"));
            let name = before.text("Українська", "English");
            ComboBox::from_id_salt("interface-language")
                .selected_text(name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut prefs.language, Language::Ukrainian, "Українська");
                    ui.selectable_value(&mut prefs.language, Language::English, "English");
                });
        });
        let mut changed = prefs.language != before;
        if let Some(desktop) = desktop {
            changed |= ui
                .add_enabled(
                    desktop.has_tray(),
                    egui::Checkbox::new(
                        &mut prefs.keep_in_tray,
                        before.text("Згортати в системний трей", "Keep in the system tray"),
                    ),
                )
                .changed();
            if desktop.has_shortcut() {
                if ui
                    .button(before.text("Вимкнути глобальну клавішу", "Disable global shortcut"))
                    .clicked()
                {
                    desktop.disable_shortcut();
                    prefs.global_shortcut = false;
                    changed = true;
                }
            } else if ui
                .button(before.text("Призначити глобальну клавішу…", "Assign global shortcut…"))
                .clicked()
            {
                desktop.enable_shortcut(before);
            }
            match &desktop.shortcut_result {
                Some(Ok(trigger)) => {
                    ui.label(format!("Sidetone · {trigger}"));
                }
                Some(Err(error)) => {
                    ui.colored_label(WARNING, error);
                }
                None if desktop.has_shortcut() => {
                    ui.spinner();
                }
                None => {}
            }
        }
        Self::save_prefs(ui, prefs, changed);
    }

    fn save_prefs(ui: &mut Ui, prefs: &Preferences, changed: bool) {
        if changed && let Err(error) = prefs.save() {
            ui.colored_label(WARNING, error);
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
