use eframe::egui::{Color32, ComboBox, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::mixer::{Channel, Config, Pair};

use super::{
    preferences::Language,
    theme::{BORDER, MUTED, TEXT},
};

pub(super) fn controls(
    ui: &mut Ui,
    config: &Config,
    dial: Option<[u8; 2]>,
    lang: Language,
) -> Option<Option<Pair>> {
    let previous = config.pair;
    let mut enabled = previous.is_some();
    let mut pair = previous.unwrap_or(Pair {
        left: Channel::Game,
        right: Channel::Media,
    });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut enabled, "ChatMix")
            .on_hover_text(lang.text("Фізичне коліщатко", "Physical dial"));
        ui.add_enabled_ui(enabled, |ui| {
            let choices = config.channels();
            ComboBox::from_id_salt("chatmix-left")
                .width(86.0)
                .selected_text(config.name(pair.left))
                .show_ui(ui, |ui| {
                    for &channel in &choices {
                        if channel != pair.right {
                            ui.selectable_value(&mut pair.left, channel, config.name(channel));
                        }
                    }
                });
            let colors = [pair.left, pair.right]
                .map(|channel| super::mixer_channels::color(config.color(channel)));
            gauge(ui, enabled.then_some(dial).flatten(), colors, lang);
            ComboBox::from_id_salt("chatmix-right")
                .width(86.0)
                .selected_text(config.name(pair.right))
                .show_ui(ui, |ui| {
                    for channel in choices {
                        if channel != pair.left {
                            ui.selectable_value(&mut pair.right, channel, config.name(channel));
                        }
                    }
                });
        });
    });
    let selected = enabled.then_some(pair);
    (selected != previous).then_some(selected)
}

fn gauge(ui: &mut Ui, dial: Option<[u8; 2]>, colors: [Color32; 2], lang: Language) {
    use eframe::egui::{Align2, FontId};

    let width = (ui.available_width() - 110.0).clamp(65.0, 440.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 27.0), Sense::hover());
    let (label, position) = match dial {
        Some([left, right]) => (
            format!("{left}% / {right}%"),
            Some((f32::from(right) - f32::from(left) + 100.0) / 200.0),
        ),
        None => ("—".into(), None),
    };
    let track = Rect::from_min_size(rect.min + vec2(0.0, 4.0), vec2(rect.width(), 5.0));
    let painter = ui.painter();
    let middle = track.center().x;
    painter.rect_filled(
        Rect::from_min_max(track.min, pos2(middle, track.bottom())),
        2,
        colors[0].gamma_multiply(0.6),
    );
    painter.rect_filled(
        Rect::from_min_max(pos2(middle, track.top()), track.max),
        2,
        colors[1].gamma_multiply(0.6),
    );
    painter.line_segment(
        [
            pos2(middle, track.top() - 2.0),
            pos2(middle, track.bottom() + 2.0),
        ],
        Stroke::new(1.0, BORDER),
    );
    if let Some(position) = position {
        let x = eframe::egui::lerp(track.x_range(), position);
        painter.rect_filled(
            Rect::from_center_size(pos2(x, track.center().y), vec2(4.0, 11.0)),
            2,
            TEXT,
        );
    }
    painter.text(
        pos2(rect.center().x, rect.bottom()),
        Align2::CENTER_BOTTOM,
        label,
        FontId::proportional(11.0),
        MUTED,
    );
    response.on_hover_text(lang.text("Коефіцієнти фізичного ChatMix", "Physical ChatMix balance"));
}
