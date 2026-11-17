mod engine;
mod layout;
mod slider;
mod theme;

use eframe::egui::ViewportBuilder;

use engine::Engine;

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_app_id("nova-control")
            .with_inner_size([780.0, 462.0])
            .with_min_inner_size([640.0, 400.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Nova Control",
        options,
        Box::new(|cc| {
            theme::configure(&cc.egui_ctx);
            Ok(Box::new(Engine::default()))
        }),
    )
}
