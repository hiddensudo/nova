use eframe::egui::{self, Align2, FontId, Slider, Ui, pos2};

use crate::device::Sidetone;

use super::{
    engine::{Engine, SliderSelection},
    theme::{ACCENT, MUTED},
};

impl Engine {
    pub(super) fn sidetone_slider(&mut self, ui: &mut Ui) -> egui::Response {
        let pending_write = self.pending_level();
        let shown = match self.slider_selection {
            Some(SliderSelection::Dragging(level) | SliderSelection::Queued(level)) => Some(level),
            None => pending_write.or(self.level),
        };

        let mut value = shown.unwrap_or(Sidetone::Off) as u8;
        let enabled = shown.is_some() && pending_write.is_none();
        let response = ui
            .scope(|ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                ui.add_enabled(
                    enabled,
                    Slider::new(&mut value, 0..=Sidetone::High as u8)
                        .show_value(false)
                        .trailing_fill(true),
                )
            })
            .inner;

        let level = Sidetone::ALL[usize::from(value)];
        if response.dragged() {
            self.drag_owner = Some(response.id);
            self.slider_selection = Some(SliderSelection::Dragging(level));
        } else if response.changed() || response.drag_stopped() {
            self.drag_owner = None;
            self.slider_selection =
                (self.level != Some(level)).then_some(SliderSelection::Queued(level));
        } else if self.drag_owner == Some(response.id) {
            self.drag_owner = None;
            self.slider_selection = None;
        }

        let (_, labels) = ui.allocate_space(egui::vec2(response.rect.width(), 18.0));
        for stop in Sidetone::ALL {
            let (x, align) = match stop {
                Sidetone::Off => (labels.left(), Align2::LEFT_CENTER),
                Sidetone::High => (labels.right(), Align2::RIGHT_CENTER),
                _ => (
                    egui::lerp(
                        labels.x_range(),
                        stop as u8 as f32 / Sidetone::High as u8 as f32,
                    ),
                    Align2::CENTER_CENTER,
                ),
            };
            let color = if shown.is_some() && stop == level {
                ACCENT
            } else {
                MUTED
            };
            ui.painter().text(
                pos2(x, labels.center().y),
                align,
                stop.label(),
                FontId::proportional(13.0),
                color,
            );
        }
        response
    }
}
