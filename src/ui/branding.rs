use std::sync::OnceLock;

use eframe::egui::{self, IconData, Painter, Pos2, Rect, TextureHandle, vec2};

const LARGE: &[u8] =
    include_bytes!("../../packaging/linux/icons/hicolor/256x256/apps/nova-control.png");
const SMALL: &[u8] =
    include_bytes!("../../packaging/linux/icons/hicolor/32x32/apps/nova-control.png");
const PANEL: &[u8] =
    include_bytes!("../../packaging/linux/icons/hicolor/64x64/apps/nova-control.png");

fn decode(data: &[u8]) -> IconData {
    let image = image::load_from_memory(data)
        .expect("embedded Nova icon must be a valid PNG")
        .to_rgba8();
    IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

pub(super) fn window_icon() -> IconData {
    static ICON: OnceLock<IconData> = OnceLock::new();
    ICON.get_or_init(|| decode(LARGE)).clone()
}

pub(super) fn tray_icons() -> Vec<ksni::Icon> {
    static ICONS: OnceLock<Vec<ksni::Icon>> = OnceLock::new();
    ICONS
        .get_or_init(|| {
            [SMALL, PANEL]
                .into_iter()
                .map(|data| {
                    let mut icon = decode(data);
                    // StatusNotifierItem requires ARGB bytes in network order, not RGBA.
                    for pixel in icon.rgba.as_chunks_mut::<4>().0 {
                        pixel.rotate_right(1);
                    }
                    ksni::Icon {
                        width: icon.width as i32,
                        height: icon.height as i32,
                        data: icon.rgba,
                    }
                })
                .collect()
        })
        .clone()
}

pub(super) fn paint(painter: &Painter, center: Pos2) {
    let id = egui::Id::unique("nova-brand-texture");
    let texture = painter
        .ctx()
        .data_mut(|data| data.get_temp::<TextureHandle>(id));
    let texture = texture.unwrap_or_else(|| {
        let icon = decode(PANEL);
        let texture = painter.ctx().load_texture(
            "nova-impulse",
            egui::ColorImage::from_rgba_unmultiplied(
                [icon.width as usize, icon.height as usize],
                &icon.rgba,
            ),
            egui::TextureOptions::LINEAR,
        );
        painter
            .ctx()
            .data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    });
    painter.image(
        texture.id(),
        Rect::from_center_size(center, vec2(30.0, 30.0)),
        Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icons_are_monochrome_and_tray_preserves_alpha_order() {
        let icon = window_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert!(
            icon.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[0] == p[1] && p[1] == p[2])
        );
        assert!(icon.rgba.as_chunks::<4>().0.iter().any(|p| p[0] == 255));
        for (source, tray) in [SMALL, PANEL].into_iter().zip(tray_icons()) {
            let source = decode(source);
            assert_eq!(tray.data.len(), (tray.width * tray.height * 4) as usize);
            for (rgba, argb) in source
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(tray.data.as_chunks::<4>().0.iter())
            {
                assert_eq!(*argb, [rgba[3], rgba[0], rgba[1], rgba[2]]);
            }
        }
    }
}
