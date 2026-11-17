use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{Context, Ui};

use crate::device::{self, Operation, Sidetone};

const READ_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(super) struct Engine {
    pub(super) level: Option<Sidetone>,
    pub(super) slider_selection: Option<SliderSelection>,
    pub(super) error: Option<String>,
    request: Option<Request>,
    next_read: Option<Instant>,
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
    fn submit(&mut self, ctx: &Context, operation: Operation) {
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

    pub(super) fn pending_level(&self) -> Option<Sidetone> {
        match self.request.as_ref()?.operation {
            Operation::Set(level) => Some(level),
            Operation::Read => None,
        }
    }
}

impl eframe::App for Engine {
    fn logic(&mut self, _ctx: &Context, _frame: &mut eframe::Frame) {
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
        self.request = None;
        self.next_read = Some(Instant::now() + READ_INTERVAL);
        match result {
            Ok(level) => {
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
