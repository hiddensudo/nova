use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

use eframe::egui::{self, Context, Ui};

use crate::mixer::{self, Channel, Config, Snapshot};

use super::{
    channel_dialog::{ChannelDialog, Interaction},
    mixer_channels::{self, Action, color},
    preferences::Language,
    theme::{MUTED, WARNING, card},
};

#[derive(Default)]
pub(super) struct MixerPage {
    worker: Option<Worker>,
    state: Option<Snapshot>,
    config: Config,
    dialog: Option<ChannelDialog>,
    icons: super::app_icons::Icons,
    error: Option<String>,
}

struct Worker {
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
    rx: mpsc::Receiver<Result<Snapshot, String>>,
}

impl Worker {
    fn start(ctx: &Context) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::sync_channel(2);
        let task = thread::spawn(move || {
            let send = |result| {
                if tx.try_send(result).is_ok() {
                    ctx.request_repaint();
                }
            };
            if let Err(error) = mixer::run(|| signal.load(Ordering::Relaxed), &send) {
                send(Err(error));
            }
            ctx.request_repaint();
        });
        Self {
            stop,
            task: Some(task),
            rx,
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}

impl MixerPage {
    pub fn new() -> Self {
        match Config::load() {
            Ok(config) => Self {
                config,
                ..Self::default()
            },
            Err(error) => Self {
                error: Some(error),
                ..Self::default()
            },
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
        if self.worker.is_none() {
            ui.label(egui::RichText::new(lang.text("Мікшер вимкнено", "Mixer off")).color(MUTED));
        }
    }

    pub(super) fn poll(&mut self) {
        let Some(worker) = &self.worker else { return };
        while let Ok(result) = worker.rx.try_recv() {
            match result {
                Ok(mut state) => {
                    if state.config.revision >= self.config.revision {
                        self.config = state.config.clone();
                    }
                    for stream in &mut state.streams {
                        stream.channel = self.config.routes.get(&stream.key).copied();
                    }
                    self.state = Some(state);
                    self.error = None;
                }
                Err(error) => {
                    self.error = Some(error);
                    self.state = None;
                }
            }
        }
        if worker.task.as_ref().is_some_and(JoinHandle::is_finished) {
            self.worker = None;
        }
    }

    pub fn show(&mut self, ui: &mut Ui, lang: Language) {
        ui.horizontal_wrapped(|ui| {
            if self.worker.is_none() {
                if ui
                    .button(lang.text("Увімкнути мікшер", "Enable mixer"))
                    .clicked()
                {
                    self.error = None;
                    self.worker = Some(Worker::start(ui.ctx()));
                }
            } else if ui
                .button(lang.text("Вимкнути мікшер", "Disable mixer"))
                .clicked()
            {
                self.worker = None;
                self.state = None;
                self.error = None;
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
        } else if self.state.is_none() && self.worker.is_some() {
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
