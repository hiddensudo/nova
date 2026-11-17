use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2,
};

use crate::device::eq::{BAND_COUNT, Band};

use super::theme::{ACCENT, BACKGROUND, BORDER, MUTED};

const MIN_HZ: f32 = 20.0;
const MAX_HZ: f32 = 20_000.0;
const GAIN: f32 = 12.0;
const LOG_SPAN: f32 = 3.0;
const SAMPLE_RATE: f64 = 48_000.0;

#[derive(Default)]
pub(super) struct Curve {
    pub selected: usize,
    dragging: Option<usize>,
}

impl Curve {
    pub fn show(&mut self, ui: &mut Ui, bands: &mut [Band; BAND_COUNT], enabled: bool) -> bool {
        let (rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), 202.0),
            if enabled {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            },
        );
        let graph = Rect::from_min_max(rect.min + vec2(39.0, 11.0), rect.max - vec2(17.0, 25.0));
        let point = |band: Band| {
            pos2(
                x_at(graph, f32::from(band.hz)),
                y_at(graph, f32::from(band.gain_tenths) / 10.0),
            )
        };
        let nearest = |pointer: Pos2, bands: &[Band; BAND_COUNT]| {
            (0..bands.len())
                .map(|index| (index, pointer.distance_sq(point(bands[index]))))
                .filter(|(_, distance)| *distance <= 14.0_f32.powi(2))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(index, _)| index)
        };
        let mut changed = false;
        if enabled && response.drag_started() {
            self.dragging = ui
                .input(|input| input.pointer.press_origin())
                .and_then(|pos| nearest(pos, bands));
        }
        if enabled
            && response.clicked()
            && let Some(index) = response
                .interact_pointer_pos()
                .and_then(|pos| nearest(pos, bands))
        {
            self.selected = index;
        }
        if enabled
            && (response.dragged() || response.drag_stopped())
            && let (Some(index), Some(pos)) = (self.dragging, response.interact_pointer_pos())
        {
            self.selected = index;
            let band = &mut bands[index];
            let (hz, gain) = values_at(graph, pos);
            changed = band.hz != hz || band.gain_tenths != gain;
            band.hz = hz;
            band.gain_tenths = gain;
        }
        if !response.dragged() {
            self.dragging = None;
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 7, BACKGROUND);
        let font = FontId::proportional(11.0);
        for gain in [-12, -6, 0, 6, 12] {
            let y = y_at(graph, gain as f32);
            painter.hline(
                graph.x_range(),
                y,
                Stroke::new(
                    1.0,
                    if gain == 0 {
                        MUTED.gamma_multiply(0.5)
                    } else {
                        BORDER
                    },
                ),
            );
            painter.text(
                pos2(graph.left() - 5.0, y),
                Align2::RIGHT_CENTER,
                gain.to_string(),
                font.clone(),
                MUTED,
            );
        }
        let labels: &[(u16, &str)] = if rect.width() < 400.0 {
            &[(20, "20"), (100, "100"), (1000, "1k"), (10_000, "10k")]
        } else {
            &[
                (20, "20"),
                (50, "50"),
                (100, "100"),
                (200, "200"),
                (500, "500"),
                (1000, "1k"),
                (2000, "2k"),
                (5000, "5k"),
                (10_000, "10k"),
                (20_000, "20k"),
            ]
        };
        for &(hz, label) in labels {
            let x = x_at(graph, f32::from(hz));
            painter.vline(x, graph.y_range(), Stroke::new(1.0, BORDER));
            painter.text(
                pos2(x, graph.bottom() + 8.0),
                Align2::CENTER_TOP,
                label,
                font.clone(),
                MUTED,
            );
        }

        let coefficients = bands.map(peak_coefficients);
        let steps = graph.width().ceil().max(2.0) as usize;
        let points: Vec<_> = (0..=steps)
            .map(|step| {
                let x = egui::lerp(graph.x_range(), step as f32 / steps as f32);
                pos2(
                    x,
                    y_at(
                        graph,
                        response_db(hz_at(graph, x), &coefficients).clamp(-GAIN, GAIN),
                    ),
                )
            })
            .collect();
        let mut mesh = egui::Mesh::default();
        let baseline = y_at(graph, 0.0);
        for (index, pos) in points.iter().enumerate() {
            mesh.colored_vertex(*pos, ACCENT.gamma_multiply(0.28));
            mesh.colored_vertex(pos2(pos.x, baseline), Color32::TRANSPARENT);
            if index > 0 {
                let first = (index as u32 - 1) * 2;
                mesh.add_triangle(first, first + 1, first + 2);
                mesh.add_triangle(first + 1, first + 2, first + 3);
            }
        }
        painter.add(Shape::mesh(mesh));
        painter.add(Shape::line(points, Stroke::new(1.8, ACCENT)));
        for (index, band) in bands.iter().enumerate() {
            let active = index == self.selected;
            painter.circle(
                point(*band),
                if active { 5.0 } else { 3.8 },
                if active { ACCENT } else { BACKGROUND },
                Stroke::new(1.1, ACCENT),
            );
        }
        changed
    }
}

fn x_at(graph: Rect, hz: f32) -> f32 {
    egui::lerp(
        graph.x_range(),
        (hz.clamp(MIN_HZ, MAX_HZ) / MIN_HZ).log10() / LOG_SPAN,
    )
}

fn hz_at(graph: Rect, x: f32) -> f32 {
    let fraction = ((x - graph.left()) / graph.width()).clamp(0.0, 1.0);
    MIN_HZ * 10.0_f32.powf(fraction * LOG_SPAN)
}

fn y_at(graph: Rect, gain: f32) -> f32 {
    egui::lerp(graph.y_range(), (GAIN - gain) / (2.0 * GAIN))
}

fn values_at(graph: Rect, pos: Pos2) -> (u16, i8) {
    let gain = (GAIN - (pos.y - graph.top()) / graph.height() * 2.0 * GAIN).clamp(-GAIN, GAIN);
    (
        hz_at(graph, pos.x).round() as u16,
        (gain * 10.0).round() as i8,
    )
}

fn peak_coefficients(band: Band) -> [f64; 5] {
    if band.filter != 1 {
        return [1.0, 0.0, 0.0, 1.0, 0.0];
    }
    let theta = std::f64::consts::TAU * f64::from(band.hz) / SAMPLE_RATE;
    let mid = -2.0 * theta.cos();
    let alpha = theta.sin() / (2.0 * f64::from(band.q_thousandths.max(200)) / 1000.0);
    let amplitude = 10.0_f64.powf(f64::from(band.gain_tenths) / 400.0);
    [
        1.0 + alpha * amplitude,
        mid,
        1.0 - alpha * amplitude,
        1.0 + alpha / amplitude,
        1.0 - alpha / amplitude,
    ]
}

fn response_db(hz: f32, coefficients: &[[f64; 5]]) -> f32 {
    let theta = std::f64::consts::TAU * f64::from(hz) / SAMPLE_RATE;
    let (s1, c1) = theta.sin_cos();
    let (s2, c2) = (2.0 * theta).sin_cos();
    coefficients
        .iter()
        .map(|&[b0, b1, b2, a0, a2]| {
            let numerator = (b0 + b1 * c1 + b2 * c2).powi(2) + (b1 * s1 + b2 * s2).powi(2);
            let denominator = (a0 + b1 * c1 + a2 * c2).powi(2) + (b1 * s1 + a2 * s2).powi(2);
            10.0 * (numerator / denominator).log10()
        })
        .sum::<f64>() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::eq::Profile;

    #[test]
    fn flat_is_flat_and_filter_peak_matches_gain() {
        let base = Profile::preset("Flat").unwrap();
        let coefficients = base.bands.map(peak_coefficients);
        for hz in [20.0, 62.0, 1000.0, 20_000.0] {
            assert!(response_db(hz, &coefficients).abs() < 0.0001);
        }
        let band = Band {
            hz: 1000,
            gain_tenths: -65,
            ..base.bands[4]
        };
        assert!((response_db(1000.0, &[peak_coefficients(band)]) + 6.5).abs() < 0.001);
    }

    #[test]
    fn drag_coordinates_respect_device_minima_maxima_and_gain_steps() {
        let rect = Rect::from_min_size(pos2(20.0, 10.0), vec2(600.0, 200.0));
        assert_eq!(values_at(rect, pos2(0.0, 0.0)), (20, 120));
        assert_eq!(values_at(rect, pos2(650.0, 220.0)), (20_000, -120));
        assert_eq!(
            values_at(rect, pos2(x_at(rect, 1500.0), y_at(rect, 2.3))),
            (1500, 23)
        );
    }
}
