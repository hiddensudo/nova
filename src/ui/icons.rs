use eframe::egui::{
    Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

use crate::mixer::ChannelIcon;

use super::theme::{ACCENT, MUTED, RAISED, SELECTED};

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Logo,
    Overview,
    Mic,
    Eq,
    Mixer,
    Headset,
    Tray,
    Speaker(bool),
    Channel(ChannelIcon),
}

pub(super) fn button(ui: &mut Ui, icon: Icon, label: &str, selected: bool, rail: bool) -> Response {
    let size = if matches!(icon, Icon::Logo) {
        vec2(40.0, 40.0)
    } else if rail {
        vec2(38.0, 38.0)
    } else {
        vec2(32.0, 30.0)
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        eframe::egui::WidgetInfo::selected(
            eframe::egui::Role::Button,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    if selected || response.hovered() || response.has_focus() {
        let fill = if selected { SELECTED } else { RAISED };
        ui.painter().rect_filled(rect, 8, fill);
    }
    if response.has_focus() {
        ui.painter()
            .rect_stroke(rect, 8, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
    }
    if selected && rail {
        let mark =
            Rect::from_center_size(pos2(rect.left() - 7.0, rect.center().y), vec2(3.0, 19.0));
        ui.painter().rect_filled(mark, 2, ACCENT);
    }
    let color = if matches!(icon, Icon::Logo) {
        Color32::WHITE
    } else if selected {
        ACCENT
    } else {
        MUTED
    };
    paint(ui.painter(), rect.center(), icon, color);
    response.on_hover_text(label)
}

pub(super) fn label(ui: &mut Ui, icon: Icon, color: Color32) {
    let (_, rect) = ui.allocate_space(vec2(20.0, 19.0));
    paint(ui.painter(), rect.center(), icon, color);
}

fn paint(painter: &Painter, center: Pos2, icon: Icon, color: Color32) {
    let stroke = Stroke::new(1.45, color);
    let point = |x, y| center + vec2(x, y);
    let line = |x, y, x2, y2| painter.line_segment([point(x, y), point(x2, y2)], stroke);
    let box_at = |x, y, w, h, radius| {
        painter.rect_stroke(
            Rect::from_min_size(point(x, y), vec2(w, h)),
            CornerRadius::same(radius),
            stroke,
            StrokeKind::Inside,
        )
    };
    match icon {
        Icon::Logo => super::branding::paint(painter, center),
        Icon::Overview => {
            for y in [-7.0, 1.0] {
                for x in [-7.0, 1.0] {
                    box_at(x, y, 6.0, 6.0, 2);
                }
            }
        }
        Icon::Mic => {
            box_at(-3.4, -8.0, 6.8, 12.0, 3);
            painter.add(eframe::egui::Shape::line(
                [
                    (-6., 0.),
                    (-6., 3.),
                    (-4., 6.),
                    (0., 7.),
                    (4., 6.),
                    (6., 3.),
                    (6., 0.),
                ]
                .map(|(x, y)| point(x, y))
                .into(),
                stroke,
            ));
            line(0., 7., 0., 9.);
        }
        Icon::Eq => {
            for (x, y) in [(-6.0, 2.0), (-2.0, 0.0), (2.0, -1.0), (6.0, -5.0)] {
                line(x, y + 3.0, x, 7.0);
            }
            painter.add(eframe::egui::Shape::line(
                [(-7., 1.), (-3., -3.), (0., -2.), (3., -5.), (7., -8.)]
                    .map(|(x, y)| point(x, y))
                    .into(),
                stroke,
            ));
        }
        Icon::Mixer => {
            for (y, knob) in [(-5.0, -3.0), (0.0, 3.0), (5.0, -1.0)] {
                line(-7.0, y, knob - 2.0, y);
                line(knob + 2.0, y, 7.0, y);
                line(knob, y - 2.0, knob, y + 2.0);
            }
        }
        Icon::Tray => {
            line(-7., 4., -7., 7.);
            line(-7., 7., 7., 7.);
            line(7., 7., 7., 4.);
            line(0., -7., 0., 3.);
            line(-3., 0., 0., 3.);
            line(0., 3., 3., 0.);
        }
        Icon::Headset => {
            painter.add(eframe::egui::Shape::line(
                [
                    (-7., 4.),
                    (-7., -2.),
                    (-5., -6.),
                    (0., -8.),
                    (5., -6.),
                    (7., -2.),
                    (7., 4.),
                ]
                .map(|(x, y)| point(x, y))
                .into(),
                stroke,
            ));
            box_at(-7.5, 0.0, 4.5, 7.0, 2);
            box_at(3.0, 0.0, 4.5, 7.0, 2);
        }
        Icon::Speaker(muted) => {
            painter.add(eframe::egui::Shape::closed_line(
                [
                    (-7., -2.),
                    (-4., -2.),
                    (0., -6.),
                    (0., 6.),
                    (-4., 2.),
                    (-7., 2.),
                ]
                .map(|(x, y)| point(x, y))
                .into(),
                stroke,
            ));
            if muted {
                line(3., -3., 8., 3.);
                line(3., 3., 8., -3.);
            } else {
                painter.add(eframe::egui::Shape::line(
                    [(3., -4.), (5., -2.), (5., 2.), (3., 4.)]
                        .map(|(x, y)| point(x, y))
                        .into(),
                    stroke,
                ));
            }
        }
        Icon::Channel(kind) => match kind {
            ChannelIcon::Game => {
                box_at(-8., -5., 16., 10., 4);
                line(-6., 0., -2., 0.);
                line(-4., -2., -4., 2.);
                painter.circle_filled(point(3., -1.), 1., color);
                painter.circle_filled(point(5., 1.), 1., color);
            }
            ChannelIcon::Music => {
                line(-2., 4., -2., -6.);
                line(5., 1., 5., -8.);
                line(-2., -6., 5., -8.);
                painter.circle_filled(point(-4., 4.), 2., color);
                painter.circle_filled(point(3., 1.), 2., color);
            }
            ChannelIcon::Chat => {
                box_at(-8., -6., 16., 11., 3);
                line(-4., 4., -6., 8.);
                line(-5., -1., 5., -1.);
            }
            ChannelIcon::Globe => {
                painter.circle_stroke(center, 7., stroke);
                line(-7., 0., 7., 0.);
                painter.add(eframe::egui::Shape::line(
                    [
                        (0., -7.),
                        (-3., -3.),
                        (-3., 3.),
                        (0., 7.),
                        (3., 3.),
                        (3., -3.),
                        (0., -7.),
                    ]
                    .map(|(x, y)| point(x, y))
                    .into(),
                    stroke,
                ));
            }
            ChannelIcon::Stream => {
                painter.circle_stroke(center, 7., stroke);
                painter.circle_stroke(center, 4., stroke);
                painter.circle_filled(center, 1.5, color);
            }
        },
    }
}
