use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{Context, Ui};

use crate::device::{self, Operation, Sidetone};

use super::eq::EqPage;

const READ_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(super) struct Engine {
    pub(super) page: Page,
    pub(super) eq: EqPage,
    pub(super) level: Option<Sidetone>,
    pub(super) slider_selection: Option<SliderSelection>,
    pub(super) error: Option<String>,
    pub(super) saved_level: Option<Sidetone>,
    request: Option<Request>,
    next_read: Option<Instant>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Page {
    #[default]
    Microphone,
    Eq,
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
    pub(super) fn submit(&mut self, ctx: &Context, operation: Operation) {
        if matches!(operation, Operation::Set(_) | Operation::Save) {
            self.saved_level = None;
        }
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
        self.level.is_some() && self.request.is_none() && self.slider_selection.is_none()
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
    fn logic(&mut self, _ctx: &Context, _frame: &mut eframe::Frame) {
        self.eq.poll();
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
        let saving = self.saving();
        self.request = None;
        self.next_read = Some(Instant::now() + READ_INTERVAL);
        match result {
            Ok(level) => {
                if saving {
                    self.saved_level = Some(level);
                } else if self.saved_level != Some(level) {
                    self.saved_level = None;
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

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
        if self.page != Page::Microphone || self.eq.busy() {
            return;
        }
        let now = Instant::now();
        if let Some(operation) = self.next_operation(now) {
            self.submit(ui.ctx(), operation);
        } else if self.request.is_none()
            && self.slider_selection.is_none()
            && let Some(deadline) = self.next_read
        {
            ui.ctx()
                .request_repaint_after(deadline.saturating_duration_since(now));
        }
    }
}

#[cfg(test)]
mod tests;
