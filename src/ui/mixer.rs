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

#[derive(Default)]
pub(super) struct MixerPage {
    worker: Option<Worker>,
    state: Option<Snapshot>,
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
    pub(super) fn poll(&mut self) {
        let Some(worker) = &self.worker else { return };
        while let Ok(result) = worker.rx.try_recv() {
            match result {
                Ok(state) => {
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

    pub(super) fn show(&mut self, ui: &mut Ui) {
        ui.heading("Мікшер");
        if self.worker.is_none() {
            if ui.button("Увімкнути Game / Media / Chat").clicked() {
                self.error = None;
                self.worker = Some(Worker::start(ui.ctx()));
            }
        } else if ui.button("Вимкнути мікшер").clicked() {
            self.worker = None;
            self.state = None;
            self.error = None;
        }
        if let Some(error) = &self.error {
            ui.colored_label(super::theme::WARNING, error);
        }
        if let Some(state) = &mut self.state {
            if let Err(error) = controls(ui, state) {
                self.error = Some(error);
            }
        } else if self.worker.is_some() && self.error.is_none() {
            ui.spinner();
        }
    }
}

fn controls(ui: &mut Ui, state: &mut Snapshot) -> Result<(), String> {
    ui.add_space(12.0);
    for channel in Channel::ALL {
        let level = &mut state.config.levels[channel as usize];
        let response = ui.add(
            egui::Slider::new(level, 0..=100)
                .suffix("%")
                .text(channel.name()),
        );
        if response.changed() {
            Config::edit(|config| config.levels[channel as usize] = *level)?;
        }
    }
    ui.separator();
    super::chatmix::controls(ui, state)?;
    ui.separator();
    ui.heading("Застосунки");
    if state.streams.is_empty() {
        ui.label("Запусти відтворення звуку в застосунку, щоб призначити йому канал.");
    }
    for stream in &mut state.streams {
        let previous = stream.channel;
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt(&stream.key)
                .selected_text(stream.channel.map(Channel::name).unwrap_or("Авто"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut stream.channel, None, "Авто");
                    for channel in Channel::ALL {
                        ui.selectable_value(&mut stream.channel, Some(channel), channel.name());
                    }
                });
            ui.label(&stream.label);
        });
        if stream.channel != previous {
            Config::edit(|config| match stream.channel {
                Some(channel) => {
                    config.routes.insert(stream.key.clone(), channel);
                }
                None => {
                    config.routes.remove(&stream.key);
                }
            })?;
        }
    }
    Ok(())
}
