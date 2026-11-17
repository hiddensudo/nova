use eframe::egui::{self, Color32, FontId, Response, Sense, Ui, pos2, vec2};

use super::theme::{ACCENT, BORDER, MUTED, TEXT};

pub(super) fn toggle(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    let label_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(label.to_owned(), FontId::proportional(14.0), TEXT)
            .size()
            .x
    });
    let (rect, mut response) =
        ui.allocate_exact_size(vec2(label_width + 55.0, 32.0), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Switch, ui.is_enabled(), *value, label)
    });
    let amount = ui.ctx().animate_bool_with_time(response.id, *value, 0.12);
    let track =
        egui::Rect::from_center_size(pos2(rect.left() + 19.0, rect.center().y), vec2(37.0, 21.0));
    let color = if *value { ACCENT } else { BORDER };
    ui.painter().rect_filled(track, 11, color);
    let x = egui::lerp(track.x_range(), amount);
    let x = x.clamp(track.left() + 10.0, track.right() - 10.0);
    ui.painter().circle_filled(
        pos2(x, track.center().y),
        7.0,
        if *value {
            Color32::from_rgb(35, 25, 19)
        } else {
            MUTED
        },
    );
    ui.painter().text(
        pos2(rect.left() + 48.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        TEXT,
    );
    response
}

pub(super) fn fader(ui: &mut Ui, value: &mut u8, max: u8, tint: Color32, width: f32) -> Response {
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = width.max(24.0);
        ui.visuals_mut().selection.bg_fill = tint;
        ui.spacing_mut().interact_size.y = 20.0;
        ui.spacing_mut().slider_rail_height = 4.0;
        ui.visuals_mut().widgets.inactive.bg_fill = BORDER;
        ui.visuals_mut().widgets.active.bg_fill = tint;
        ui.visuals_mut().widgets.inactive.fg_stroke.color = tint;
        ui.add(
            egui::Slider::new(value, 0..=max)
                .show_value(false)
                .handle_shape(egui::style::HandleShape::Circle)
                .trailing_fill(true),
        )
    })
    .inner
}
