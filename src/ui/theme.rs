use std::fs;

use eframe::egui::{self, Color32, Context, FontData, FontDefinitions, FontFamily};

pub(super) const BACKGROUND: Color32 = Color32::from_rgb(22, 23, 26);
pub(super) const SIDEBAR: Color32 = Color32::from_rgb(17, 18, 20);
pub(super) const CARD: Color32 = Color32::from_rgb(32, 34, 38);
pub(super) const BORDER: Color32 = Color32::from_rgb(54, 56, 62);
pub(super) const TEXT: Color32 = Color32::from_rgb(243, 244, 247);
pub(super) const MUTED: Color32 = Color32::from_rgb(164, 168, 178);
pub(super) const ACCENT: Color32 = Color32::from_rgb(255, 111, 61);
pub(super) const SUCCESS: Color32 = Color32::from_rgb(139, 221, 175);
pub(super) const WARNING: Color32 = Color32::from_rgb(255, 179, 152);

pub(super) fn configure(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BACKGROUND;
    visuals.widgets.inactive.bg_fill = SIDEBAR;
    visuals.widgets.inactive.weak_bg_fill = SIDEBAR;
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(48, 49, 54);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(59, 49, 47);
    visuals.selection.bg_fill = ACCENT;
    ctx.set_visuals(visuals);

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
