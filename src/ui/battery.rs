use std::time::Duration;

use eframe::egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub(super) fn icon(ui: &mut Ui, percent: u8, charging: bool, color: Color32) {
    let (bounds, response) = ui.allocate_exact_size(vec2(25.0, 21.0), Sense::hover());
    let body = Rect::from_center_size(bounds.center() - vec2(1.0, 0.0), vec2(19.0, 11.0));
    let painter = ui.painter();
    painter.rect_stroke(body, 3, Stroke::new(1.2, color), StrokeKind::Inside);
    painter.rect_filled(
        Rect::from_center_size(pos2(body.right() + 1.8, body.center().y), vec2(2.0, 4.0)),
        1,
        color,
    );
    let fraction = if charging {
        ui.ctx().request_repaint_after(Duration::from_millis(33));
        let time = ui.input(|input| input.time) as f32;
        0.12 + 0.88 * (time / 1.6).fract()
    } else {
        f32::from(percent) / 100.0
    };
    let inside = body.shrink(3.0);
    painter.rect_filled(
        Rect::from_min_size(inside.min, vec2(inside.width() * fraction, inside.height())),
        1,
        color,
    );
    if charging {
        let c = body.center();
        painter.line_segment(
            [c + vec2(1., -3.), c + vec2(-1., 0.)],
            Stroke::new(1.3, super::theme::TEXT),
        );
        painter.line_segment(
            [c + vec2(-1., 0.), c + vec2(2., 0.)],
            Stroke::new(1.3, super::theme::TEXT),
        );
        painter.line_segment(
            [c + vec2(2., 0.), c + vec2(0., 3.)],
            Stroke::new(1.3, super::theme::TEXT),
        );
    }
    response.on_hover_text(format!("{percent}%"));
}
