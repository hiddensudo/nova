use super::*;
use eframe::egui::{Context, Pos2, RawInput, Rect, Shape};

#[test]
fn names_share_a_left_edge_and_channel_menus_stay_inside_the_card() {
    let ctx = Context::default();
    super::super::theme::configure(&ctx);
    let config = Config::default();
    let streams: Vec<_> = ["Spotify", "PathOfExileSteam.exe"]
        .into_iter()
        .map(|label| AppStream {
            key: label.into(),
            label: label.into(),
            icon_name: None,
            binary: None,
            channel: Some(crate::mixer::Channel::Media),
            playing: true,
        })
        .collect();
    let mut icons = Icons::default();
    for width in [350.0, 620.0, 1000.0] {
        let raw = || RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 300.0))),
            ..Default::default()
        };
        ctx.run_ui(raw(), |ui| {
            show(ui, &config, &streams, &mut icons, Language::Ukrainian);
        })
        .textures_delta
        .clear();
        let mut output = ctx.run_ui(raw(), |ui| {
            show(ui, &config, &streams, &mut icons, Language::Ukrainian);
        });
        output.textures_delta.clear();
        let text: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let Shape::Text(text) = &shape.shape {
                    Some(text)
                } else {
                    None
                }
            })
            .collect();
        let names: Vec<_> = text
            .iter()
            .filter(|s| streams.iter().any(|app| app.label == s.galley.text()))
            .collect();
        assert_eq!(names.len(), 2);
        assert!((names[0].pos.x - names[1].pos.x).abs() < 1.0);
        assert!(
            names[0].pos.x < 80.0,
            "names should start at the left, width {width}"
        );
        assert!(text.iter().all(|t| !t.galley.text().contains('⠿')));
        let menus: Vec<_> = text.iter().filter(|s| s.galley.text() == "Media").collect();
        assert_eq!(menus.len(), 2);
        for (name, menu) in names.iter().zip(menus) {
            assert!(name.visual_bounding_rect().right() < menu.visual_bounding_rect().left());
            assert!(menu.visual_bounding_rect().right() <= width);
        }
    }
}
