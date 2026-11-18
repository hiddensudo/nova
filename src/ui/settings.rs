use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{ComboBox, Context, Ui};

use crate::device::{
    self,
    settings::{self, Change, Settings},
};

use super::{
    notifications::Notifications,
    preferences::{Language, Preferences},
    theme::{WARNING, card},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Connection {
    #[default]
    Checking,
    Connected,
    HeadsetDisconnected,
    DongleMissing,
    ControlsUnavailable,
}

#[derive(Default)]
pub(super) struct SettingsPage {
    state: Option<Settings>,
    connection: Connection,
    request: Option<mpsc::Receiver<Result<Settings, String>>>,
    error: Option<String>,
    next_read: Option<Instant>,
    full_read: Option<Instant>,
    notices: Notifications,
    gain_drag: Option<u8>,
    writing: bool,
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
        self.next_read =
            Some(Instant::now() + Duration::from_millis(if result.is_ok() { 350 } else { 1500 }));
        self.request = None;
        let was_writing = std::mem::take(&mut self.writing);
        let dongle_present = result.is_ok() || device::dongle_present();
        self.update_status(result, dongle_present, was_writing, prefs);
    }

    fn update_status(
        &mut self,
        result: Result<Settings, String>,
        dongle_present: bool,
        was_writing: bool,
        prefs: &Preferences,
    ) {
        match result {
            Ok(state) => {
                self.notices.observe(state, prefs);
                self.connection = if state.connected {
                    Connection::Connected
                } else {
                    Connection::HeadsetDisconnected
                };
                self.state = Some(state);
                self.error = None;
            }
            Err(error) => {
                let disconnected = !dongle_present;
                self.notices.error(disconnected, &error, prefs);
                if disconnected || !was_writing {
                    self.state = None;
                    self.connection = if disconnected {
                        Connection::DongleMissing
                    } else {
                        Connection::ControlsUnavailable
                    };
                }
                self.error = Some(error);
            }
        }
    }

    pub fn busy(&self) -> bool {
        self.request.is_some()
    }

    pub fn writing(&self) -> bool {
        self.writing
    }

    pub fn data(&self) -> Option<Settings> {
        self.state
    }

    pub fn connection(&self) -> Connection {
        self.connection
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn refresh(&mut self, ctx: &Context) {
        if self.busy() {
            return;
        }
        let now = Instant::now();
        match self.next_read {
            Some(deadline) if deadline > now => ctx.request_repaint_after(deadline - now),
            _ => {
                let previous = self.state;
                let full =
                    previous.is_none() || self.full_read.is_none_or(|deadline| now >= deadline);
                if full {
                    self.full_read = Some(now + Duration::from_secs(15));
                }
                self.run(ctx, move || match previous {
                    Some(state) if !full => settings::refresh_status(state),
                    _ => settings::current(),
                });
            }
        }
    }

    pub fn gain(&mut self, ui: &mut Ui, lang: Language, device_busy: bool) {
        let title = lang.text("Підсилення мікрофона", "Mic gain");
        ui.strong(title);
        let mut value = self
            .gain_drag
            .or(self.state.map(|state| state.mic_volume))
            .unwrap_or(0);
        let width = (ui.available_width() - 49.0).max(60.0);
        let response = ui
            .add_enabled_ui(self.enabled(device_busy), |ui| {
                ui.horizontal(|ui| {
                    let response =
                        super::controls::fader(ui, &mut value, 7, super::theme::ACCENT, width);
                    ui.label(format!("{value}/7"));
                    response
                })
                .inner
            })
            .inner;
        if response.dragged() {
            self.gain_drag = Some(value);
        } else if response.drag_stopped() || response.changed() {
            self.gain_drag = None;
            let change = Change::MicVolume(value);
            self.writing = true;
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
        ui.columns(2, |columns| {
            card(&mut columns[0], |ui| {
                ui.set_min_height(162.0);
                self.hardware(ui, lang, device_busy);
            });
            card(&mut columns[1], |ui| {
                ui.set_min_height(162.0);
                Self::notifications(ui, prefs);
            });
        });
        ui.add_space(10.0);
        card(ui, |ui| Self::application(ui, prefs, desktop));
    }

    fn enabled(&self, device_busy: bool) -> bool {
        !self.writing
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
                    self.writing = true;
                    self.run(ui.ctx(), move || settings::apply(change));
                } else if ui
                    .button(lang.text("Зберегти в гарнітурі", "Save to headset"))
                    .clicked()
                {
                    self.writing = true;
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
        ui.add_space(5.0);
        let mut changed = super::controls::toggle(
            ui,
            &mut prefs.low_battery,
            lang.text("Низький заряд", "Low battery"),
        )
        .changed();
        changed |= super::controls::toggle(
            ui,
            &mut prefs.disconnected,
            lang.text("Від’єднання гарнітури", "Headset disconnected"),
        )
        .changed();
        changed |= super::controls::toggle(
            ui,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_controls_do_not_mean_the_headset_or_dongle_disconnected() {
        let mut page = SettingsPage::default();
        let prefs = Preferences {
            errors: false,
            disconnected: false,
            ..Preferences::default()
        };
        let state = Settings {
            connected: true,
            battery: Some(71),
            charging: settings::ChargeState::Discharging,
            mic_volume: 7,
            sleep_minutes: 30,
            mute_led: 1,
        };

        page.update_status(Ok(state), true, false, &prefs);
        assert_eq!(page.connection(), Connection::Connected);

        page.update_status(
            Err("Device or resource busy (os error 16)".into()),
            true,
            false,
            &prefs,
        );
        assert_eq!(page.connection(), Connection::ControlsUnavailable);
        assert!(page.data().is_none());
        assert!(!page.enabled(false));

        page.update_status(Ok(state), true, false, &prefs);
        assert_eq!(page.connection(), Connection::Connected);
        assert_eq!(page.data(), Some(state));
        assert!(page.enabled(false));

        page.update_status(
            Ok(Settings {
                connected: false,
                ..state
            }),
            true,
            false,
            &prefs,
        );
        assert_eq!(page.connection(), Connection::HeadsetDisconnected);

        page.update_status(Err("USB device gone".into()), false, false, &prefs);
        assert_eq!(page.connection(), Connection::DongleMissing);
    }
}
