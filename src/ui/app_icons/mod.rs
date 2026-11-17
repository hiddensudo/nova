mod desktop;
mod theme;

use std::{collections::HashMap, fs, io::Cursor, path::Path, sync::mpsc, thread};

use eframe::egui::{self, ColorImage, TextureHandle, Ui, vec2};

use crate::mixer::AppStream;

use super::theme::{MUTED, RAISED};
use desktop::{Query, Resolver};

#[derive(Default)]
pub(super) struct Icons {
    cache: HashMap<Query, Option<TextureHandle>>,
    loader: Option<Loader>,
}

struct Loader {
    requests: mpsc::Sender<Query>,
    results: mpsc::Receiver<(Query, Option<ColorImage>)>,
}

impl Loader {
    fn new(ctx: &egui::Context) -> Self {
        let (requests, pending) = mpsc::channel::<Query>();
        let (complete, results) = mpsc::channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let resolver = Resolver::new();
            while let Ok(query) = pending.recv() {
                let image = resolver.find(&query).and_then(|path| load(&path));
                if complete.send((query, image)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { requests, results }
    }
}

impl Icons {
    pub fn show(&mut self, ui: &mut Ui, stream: &AppStream) {
        let query = Query::from(stream);
        let loader = self.loader.get_or_insert_with(|| Loader::new(ui.ctx()));
        while let Ok((key, image)) = loader.results.try_recv() {
            let texture = image.map(|image| {
                ui.ctx().load_texture(
                    format!("app-icon:{}", key.key),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
            self.cache.insert(key, texture);
        }
        if !self.cache.contains_key(&query) {
            self.cache.insert(query.clone(), None);
            let _ = loader.requests.send(query.clone());
        }
        let (rect, _) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::hover());
        if let Some(Some(texture)) = self.cache.get(&query) {
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            ui.painter().rect_filled(rect, 6, RAISED);
            let initial = stream
                .label
                .chars()
                .find(|c| c.is_alphanumeric())
                .unwrap_or('?');
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                initial.to_uppercase().to_string(),
                egui::FontId::proportional(13.0),
                MUTED,
            );
        }
    }
}

fn load(path: &Path) -> Option<ColorImage> {
    if fs::metadata(path).ok()?.len() > 4 * 1024 * 1024 {
        return None;
    }
    let data = fs::read(path).ok()?;
    if path.extension().is_some_and(|ext| ext == "svg") {
        let options = resvg::usvg::Options::default();
        let tree = resvg::usvg::Tree::from_data(&data, &options).ok()?;
        let mut pixels = resvg::tiny_skia::Pixmap::new(64, 64)?;
        let size = tree.size();
        let scale = (64.0 / size.width()).min(64.0 / size.height());
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale).post_translate(
            (64.0 - size.width() * scale) / 2.0,
            (64.0 - size.height() * scale) / 2.0,
        );
        resvg::render(&tree, transform, &mut pixels.as_mut());
        Some(ColorImage::from_rgba_premultiplied([64, 64], pixels.data()))
    } else {
        let mut reader = image::ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(2048);
        limits.max_image_height = Some(2048);
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().ok()?.thumbnail(64, 64).to_rgba8();
        Some(ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ))
    }
}

#[cfg(test)]
mod tests;
