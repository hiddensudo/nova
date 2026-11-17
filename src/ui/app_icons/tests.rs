use super::*;
#[test]
fn decodes_png_and_svg_icons_with_transparency_and_rejects_bad_data() {
    let root = std::env::temp_dir().join(format!("nova-icon-decode-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let svg = root.join("icon.svg");
    fs::write(&svg, r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="16" height="32" fill="#ff0000"/></svg>"##).unwrap();
    let loaded = load(&svg).unwrap();
    assert_eq!(loaded.size, [64, 64]);
    assert_eq!(loaded.pixels[16], egui::Color32::RED);
    assert_eq!(loaded.pixels[63], egui::Color32::TRANSPARENT);
    let png = root.join("icon.png");
    image::RgbaImage::from_pixel(32, 32, image::Rgba([0, 255, 0, 255]))
        .save(&png)
        .unwrap();
    assert!(
        load(&png)
            .unwrap()
            .pixels
            .iter()
            .all(|p| *p == egui::Color32::GREEN)
    );
    fs::write(&png, "broken icon").unwrap();
    assert!(load(&png).is_none());
    fs::remove_dir_all(root).unwrap();
}
