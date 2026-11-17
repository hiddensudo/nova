use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{Context, Id, Ui};

use crate::device::{self, Operation, Sidetone};

use super::{eq::EqPage, mixer::MixerPage, preferences::Preferences, settings::SettingsPage};

const READ_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(super) struct Engine {
    pub(super) page: Page,
    pub(super) navigation: super::navigation::Navigation,
    pub(super) eq: EqPage,
    pub(super) mixer: MixerPage,
    pub(super) settings: SettingsPage,
    pub(super) prefs: Preferences,
    pub(super) desktop: Option<super::desktop::Desktop>,
    pub(super) drag_owner: Option<Id>,
    pub(super) level: Option<Sidetone>,
    pub(super) slider_selection: Option<SliderSelection>,
    pub(super) error: Option<String>,
    pub(super) saved_level: Option<Sidetone>,
    request: Option<Request>,
    next_read: Option<Instant>,
    pub(super) instance: Option<super::instance::Instance>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Page {
    #[default]
    Overview,
    Eq,
    Mixer,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SliderSelection {
    Dragging(Sidetone),
    Queued(Sidetone),
}

struct Request {
    operation: Operation,
    result: mpsc::Receiver<Result<Sidetone, String>>,
}

impl Engine {
    pub fn new(ctx: &Context) -> Self {
        let prefs = Preferences::load();
        let mut desktop = super::desktop::Desktop::new(ctx, prefs.language);
        if prefs.global_shortcut {
            desktop.enable_shortcut(prefs.language);
        }
        Self {
            prefs,
            desktop: Some(desktop),
            mixer: MixerPage::new(),
            ..Self::default()
        }
    }

    pub(super) fn submit(&mut self, ctx: &Context, operation: Operation) {
        let (send, receive) = mpsc::channel();
        self.request = Some(Request {
            operation,
            result: receive,
        });
        ctx.request_repaint();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = send.send(device::sidetone(operation));
            ctx.request_repaint();
        });
    }

    fn next_operation(&mut self, now: Instant) -> Option<Operation> {
        if self.request.is_some() {
            return None;
        }
        match self.slider_selection {
            Some(SliderSelection::Dragging(_)) => return None,
            Some(SliderSelection::Queued(level)) => {
                self.slider_selection = None;
                if self.level != Some(level) {
                    return Some(Operation::Set(level));
                }
            }
            None => {}
        }
        self.next_read
            .is_none_or(|deadline| now >= deadline)
            .then_some(Operation::Read)
    }

    pub(super) fn mic_busy(&self) -> bool {
        self.request.is_some()
    }

    pub(super) fn can_save(&self) -> bool {
        self.level.is_some()
            && self.level != self.saved_level
            && self.request.is_none()
            && self.slider_selection.is_none()
    }

    pub(super) fn saving(&self) -> bool {
        matches!(
            self.request.as_ref().map(|r| r.operation),
            Some(Operation::Save)
        )
    }

    pub(super) fn pending_level(&self) -> Option<Sidetone> {
        match self.request.as_ref()?.operation {
            Operation::Set(level) => Some(level),
            Operation::Save => self.level,
            Operation::Read => None,
        }
    }
}

impl eframe::App for Engine {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.eq.poll();
        self.mixer.poll();
        self.settings.poll(&self.prefs);
        self.poll_sidetone();
        if let Some(desktop) = &mut self.desktop {
            if self
                .instance
                .as_ref()
                .is_some_and(|instance| instance.activated())
            {
                desktop.show(ctx);
            }
            let actions = desktop.poll(ctx, self.prefs.language, self.level);
            let hidden = desktop.hidden;
            if !self.prefs.global_shortcut
                && desktop.shortcut_result.as_ref().is_some_and(Result::is_ok)
            {
                self.prefs.global_shortcut = true;
                let _ = self.prefs.save();
            }
            for action in actions {
                let level = match action {
                    super::desktop::Action::Sidetone(level) => Some(level),
                    super::desktop::Action::Cycle => self
                        .slider_selection
                        .map(|selection| match selection {
                            SliderSelection::Queued(level) | SliderSelection::Dragging(level) => {
                                level
                            }
                        })
                        .or_else(|| self.pending_level())
                        .or(self.level)
                        .map(|current| Sidetone::ALL[(current as usize + 1) % Sidetone::ALL.len()]),
                    _ => None,
                };
                if let Some(level) = level {
                    self.slider_selection = Some(SliderSelection::Queued(level));
                    self.drag_owner = None;
                }
            }
            if hidden {
                self.schedule(ctx);
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
        self.schedule(ui.ctx());
    }
}

impl Engine {
    fn poll_sidetone(&mut self) {
        let Some(request) = &self.request else {
            return;
        };
        let result = match request.result.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Запит до донгла перервався. Рівень буде зчитано повторно.".into())
            }
        };
        let operation = request.operation;
        self.request = None;
        self.next_read = Some(Instant::now() + READ_INTERVAL);
        match result {
            Ok(level) => {
                if matches!(operation, Operation::Save)
                    || matches!(operation, Operation::Read)
                        && (self.saved_level.is_none()
                            || self.level.is_some_and(|prior| prior != level))
                {
                    self.saved_level = Some(level);
                }
                self.level = Some(level);
                self.error = None;
            }
            Err(error) => {
                self.level = None;
                self.error = Some(error);
            }
        }
    }

    fn schedule(&mut self, ctx: &Context) {
        if self.eq.busy() || self.settings.busy() {
            return;
        }
        let now = Instant::now();
        if let Some(operation) = self.next_operation(now) {
            self.submit(ctx, operation);
        } else if self.request.is_none() && self.slider_selection.is_none() {
            if self.page == Page::Overview
                && self.settings.data().is_some_and(|state| state.connected)
            {
                self.eq.ensure_loaded(ctx);
            }
            if !self.eq.busy() {
                self.settings.refresh(ctx);
            }
            if let Some(deadline) = self.next_read {
                ctx.request_repaint_after(deadline.saturating_duration_since(now));
            }
        }
    }
}

#[cfg(test)]
mod tests;
