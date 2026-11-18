use std::os::unix::process::CommandExt;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{self, Context, Ui};

use crate::mixer::{Channel, Config, Snapshot, background};

use super::{
    channel_dialog::{ChannelDialog, Interaction},
    mixer_channels::{self, Action, color},
    preferences::Language,
    theme::{MUTED, WARNING, card},
};

#[derive(Default)]
pub(super) struct MixerPage {
    enabled: bool,
    status_path: Option<PathBuf>,
    status_bytes: Option<Vec<u8>>,
    next_poll: Option<Instant>,
    state: Option<Snapshot>,
    config: Config,
    dialog: Option<ChannelDialog>,
    icons: super::app_icons::Icons,
    error: Option<String>,
}

impl MixerPage {
    pub fn new() -> Self {
        let mut page = Self {
            status_path: background::status_path().ok(),
            ..Self::default()
        };
        match Config::load() {
            Ok(config) => page.config = config,
            Err(error) => page.error = Some(error),
        }
        match background::enabled() {
            Ok(enabled) => {
                page.enabled = enabled;
                if enabled {
                    page.start_background();
                }
            }
            Err(error) => page.error = Some(error),
        }
        page
    }

    fn start_background(&mut self) {
        let child = std::env::current_exe().and_then(|binary| {
            Command::new(binary)
                .args(["mixer", "background"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
        });
        match child {
            Ok(mut child) => {
                // Reap duplicate launches while letting the session process
                // keep running after the window closes.
                thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        if let Err(error) = background::set_enabled(enabled) {
            self.error = Some(error);
            return;
        }
        self.enabled = enabled;
        self.error = None;
        self.state = None;
        self.status_bytes = None;
        self.next_poll = None;
        if enabled {
            self.start_background();
        }
    }

    pub fn summary(&self, ui: &mut Ui, lang: Language) {
        for channel in self.config.channels().into_iter().take(3) {
            let percent = self.config.level(channel);
            let tint = color(self.config.color(channel));
            ui.horizontal(|ui| {
                super::icons::label(
                    ui,
                    super::icons::Icon::Channel(self.config.icon(channel)),
                    tint,
                );
                ui.add_sized(
                    [65.0, 19.0],
                    egui::Label::new(self.config.name(channel)).truncate(),
                );
                ui.add(
                    egui::ProgressBar::new(f32::from(percent) / 100.0)
                        .desired_width((ui.available_width() - 48.0).max(30.0))
                        .desired_height(5.0)
                        .fill(tint),
                );
                ui.label(format!("{percent}%"));
            });
        }
        if !self.enabled {
            ui.label(egui::RichText::new(lang.text("Мікшер вимкнено", "Mixer off")).color(MUTED));
        }
    }

    pub(super) fn poll(&mut self, ctx: &Context, show_dial: bool) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        if let Some(deadline) = self.next_poll.filter(|deadline| *deadline > now) {
            ctx.request_repaint_after(deadline - now);
            return;
        }
        let interval = Duration::from_millis(if show_dial { 25 } else { 350 });
        self.next_poll = Some(now + interval);
        ctx.request_repaint_after(interval);
        let bytes = self
            .status_path
            .as_ref()
            .and_then(|path| fs::read(path).ok());
        if bytes == self.status_bytes {
            return;
        }
        self.status_bytes = bytes;
        let status = self
            .status_bytes
            .as_deref()
            .and_then(|bytes| serde_json::from_slice::<Result<Snapshot, String>>(bytes).ok());
        match status {
            Some(Ok(mut state)) => {
                if state.config.revision >= self.config.revision {
                    self.config = state.config.clone();
                }
                for stream in &mut state.streams {
                    stream.channel = self.config.routes.get(&stream.key).copied();
                }
                self.state = Some(state);
                self.error = None;
            }
            Some(Err(error)) => {
                self.error = Some(error);
                self.state = None;
            }
            None => self.state = None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, lang: Language) {
        ui.horizontal_wrapped(|ui| {
            let enabled = self.enabled;
            let label = if enabled {
                lang.text("Вимкнути мікшер", "Disable mixer")
            } else {
                lang.text("Увімкнути мікшер", "Enable mixer")
            };
            if ui.button(label).clicked() {
                self.set_enabled(!enabled);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(lang.text("+  Додати канал", "+  Add channel"))
                    .clicked()
                {
                    self.dialog = Some(ChannelDialog::new(None, &self.config));
                }
            });
        });
        if let Some(error) = &self.error {
            ui.colored_label(WARNING, error);
        } else if self.state.is_none() && self.enabled {
            ui.spinner();
        }
        let dial = self.state.as_ref().and_then(|state| state.dial);
        let pair = card(ui, |ui| {
            super::chatmix::controls(ui, &self.config, dial, lang)
        })
        .inner;
        if let Some(pair) = pair {
            self.update(|config| {
                config.pair = pair;
                Ok(())
            });
        }
        ui.add_space(5.0);
        let active = self
            .state
            .as_ref()
            .map_or([].as_slice(), |state| state.streams.as_slice());
        if let Some(action) = mixer_channels::split(ui, &self.config, active, &mut self.icons, lang)
        {
            self.apply(action);
        }
        self.channel_dialog(ui, lang);
    }

    fn channel_dialog(&mut self, ui: &mut Ui, lang: Language) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let Some(action) = dialog.show(ui, &self.config, lang) else {
            self.dialog = Some(dialog);
            return;
        };
        if !matches!(action, Interaction::Cancel) {
            match Config::change(|config| {
                dialog.apply(config, matches!(action, Interaction::Delete))?;
                Ok(config.clone())
            }) {
                Ok(config) => self.config = config,
                Err(error) => {
                    dialog.error = Some(error);
                    self.dialog = Some(dialog);
                }
            }
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Level(channel, level) => {
                self.update(|config| config.set_level(channel, level));
            }
            Action::Mute(channel, muted) => {
                self.update(|config| {
                    config.set_mute(channel, muted);
                    Ok(())
                });
            }
            Action::Color(channel, color) => {
                self.update(|config| config.set_color(channel, color));
            }
            Action::Reorder(channel, target) => {
                self.update(|config| config.move_channel(channel, target));
            }
            Action::Edit(channel @ Channel::Custom(_)) => {
                self.dialog = Some(ChannelDialog::new(Some(channel), &self.config))
            }
            Action::Edit(_) => {}
            Action::Route(key, value) => {
                let changed = self.update(|config| {
                    if let Some(channel) = value {
                        config.routes.insert(key.clone(), channel);
                    } else {
                        config.routes.remove(&key);
                    }
                    Ok(())
                });
                if changed
                    && let Some(stream) = self
                        .state
                        .as_mut()
                        .and_then(|state| state.streams.iter_mut().find(|app| app.key == key))
                {
                    stream.channel = value;
                }
            }
        }
    }

    fn update(&mut self, edit: impl FnOnce(&mut Config) -> Result<(), String>) -> bool {
        match Config::change(|config| {
            edit(config)?;
            Ok(config.clone())
        }) {
            Ok(config) => {
                self.config = config;
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }
}
