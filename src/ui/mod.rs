mod app_icons;
mod applications;
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
mod instance;
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
    let Some(mut instance) = instance::Instance::acquire()
        .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?
    else {
        return Ok(());
    };
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
        Box::new(move |cc| {
            theme::configure(&cc.egui_ctx);
            instance.listen(&cc.egui_ctx)?;
            let mut engine = Engine::new(&cc.egui_ctx);
            engine.instance = Some(instance);
            Ok(Box::new(engine))
        }),
    )
}
