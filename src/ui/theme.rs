use std::fs;

use eframe::egui::{
    self, Color32, Context, FontData, FontDefinitions, FontFamily, Frame, Margin, Stroke, Ui, vec2,
};

pub(super) const BACKGROUND: Color32 = Color32::from_rgb(26, 27, 31);
pub(super) const SIDEBAR: Color32 = Color32::from_rgb(20, 21, 25);
pub(super) const CARD: Color32 = Color32::from_rgb(35, 37, 42);
pub(super) const RAISED: Color32 = Color32::from_rgb(44, 46, 52);
pub(super) const SELECTED: Color32 = Color32::from_rgb(56, 44, 38);
pub(super) const BORDER: Color32 = Color32::from_rgb(55, 58, 67);
pub(super) const TEXT: Color32 = Color32::from_rgb(242, 242, 245);
pub(super) const MUTED: Color32 = Color32::from_rgb(174, 177, 190);
pub(super) const ACCENT: Color32 = Color32::from_rgb(255, 155, 84);
pub(super) const SUCCESS: Color32 = Color32::from_rgb(121, 213, 170);
pub(super) const WARNING: Color32 = Color32::from_rgb(255, 169, 151);

pub(super) fn card<R>(ui: &mut Ui, contents: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10)
        .inner_margin(Margin::same(13))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            contents(ui)
        })
}

pub(super) fn configure(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.widgets.inactive.bg_fill = RAISED;
    visuals.widgets.inactive.weak_bg_fill = RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(54, 56, 64);
    visuals.widgets.active.weak_bg_fill = SELECTED;
    visuals.selection.bg_fill = SELECTED;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_visuals(visuals);

    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.interact_size.y = 25.0;
        style.spacing.button_padding = vec2(8.0, 4.0);
    });

    if let Ok(inter) = fs::read("/usr/share/fonts/inter/InterVariable.ttf") {
        let mut fonts = FontDefinitions::default();
        fonts
            .font_data
            .insert("Inter".into(), FontData::from_owned(inter).into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "Inter".into());
        ctx.set_fonts(fonts);
    }
}
