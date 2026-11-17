mod shortcut;
mod tray;

use std::{sync::mpsc, thread};

use eframe::egui::{Context, ViewportCommand};

use crate::device::Sidetone;

use super::preferences::Language;

pub(super) enum Action {
    Show,
    Quit,
    Sidetone(Sidetone),
    Cycle,
}

enum Event {
    Action(Action),
    Tray(ksni::blocking::Handle<tray::Tray>),
    ShortcutReady(String),
    ShortcutError(String),
}

#[derive(Clone)]
struct Signal {
    tx: mpsc::Sender<Event>,
    ctx: Context,
}

impl Signal {
    fn send(&self, event: Event) {
        if self.tx.send(event).is_ok() {
            self.ctx.request_repaint();
        }
    }
}

pub(super) struct Desktop {
    tx: Signal,
    rx: mpsc::Receiver<Event>,
    tray: Option<ksni::blocking::Handle<tray::Tray>>,
    tray_display: Option<(Language, Option<Sidetone>)>,
    shortcut_stop: Option<async_channel::Sender<()>>,
    pub shortcut_result: Option<Result<String, String>>,
    pub hidden: bool,
    exiting: bool,
}

impl Desktop {
    pub fn new(ctx: &Context, language: Language) -> Self {
        let (tx, rx) = mpsc::channel();
        let tx = Signal {
            tx,
            ctx: ctx.clone(),
        };
        let startup = tx.clone();
        thread::spawn(move || tray::register(startup, language));
        Self {
            tx,
            rx,
            tray: None,
            tray_display: None,
            shortcut_stop: None,
            shortcut_result: None,
            hidden: false,
            exiting: false,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        self.hidden = false;
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    pub fn minimize(&mut self, ctx: &Context) {
        if self.has_tray() {
            self.hidden = true;
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
    }

    pub fn has_tray(&self) -> bool {
        self.tray.as_ref().is_some_and(|handle| !handle.is_closed())
    }

    pub fn has_shortcut(&self) -> bool {
        self.shortcut_stop.is_some()
    }

    pub fn enable_shortcut(&mut self, language: Language) {
        if self.shortcut_stop.is_none() {
            let (stop, cancel) = async_channel::bounded(1);
            self.shortcut_stop = Some(stop);
            self.shortcut_result = None;
            let tx = self.tx.clone();
            thread::spawn(move || shortcut::register(tx, cancel, language));
        }
    }

    pub fn disable_shortcut(&mut self) {
        if let Some(stop) = self.shortcut_stop.take() {
            stop.close();
        }
        self.shortcut_result = None;
    }

    pub fn poll(&mut self, ctx: &Context, lang: Language, level: Option<Sidetone>) -> Vec<Action> {
        let mut actions = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Action(Action::Show) => {
                    self.show(ctx);
                }
                Event::Action(Action::Quit) => {
                    self.exiting = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                Event::Action(action) => actions.push(action),
                Event::Tray(handle) => self.tray = Some(handle),
                Event::ShortcutReady(trigger) if self.has_shortcut() => {
                    self.shortcut_result = Some(Ok(trigger))
                }
                Event::ShortcutError(error) if self.has_shortcut() => {
                    self.disable_shortcut();
                    self.shortcut_result = Some(Err(error));
                }
                _ => {}
            }
        }
        if let Some(tray) = &self.tray
            && self.tray_display != Some((lang, level))
        {
            tray.update(|tray| {
                tray.language = lang;
                tray.level = level;
            });
            self.tray_display = Some((lang, level));
        }
        if self.hidden && !self.has_tray() && !self.exiting {
            self.hidden = false;
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        }
        actions
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        self.disable_shortcut();
        if let Some(tray) = self.tray.take() {
            let _ = tray.shutdown();
        }
    }
}
