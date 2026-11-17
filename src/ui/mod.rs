mod battery;
mod channel_dialog;
mod chatmix;
mod controls;
mod desktop;
mod engine;
mod eq;
mod eq_curve;
mod eq_editor;
mod icons;
mod layout;
mod mixer;
mod mixer_channels;
mod navigation;
mod notifications;
mod overview;
mod preferences;
mod settings;
mod slider;
mod theme;

use eframe::egui::ViewportBuilder;

use engine::Engine;

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_app_id("nova-control")
            .with_inner_size([1000.0, 650.0])
            .with_min_inner_size([620.0, 420.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Nova Control",
        options,
        Box::new(|cc| {
            theme::configure(&cc.egui_ctx);
            Ok(Box::new(Engine::new(&cc.egui_ctx)))
        }),
    )
}
