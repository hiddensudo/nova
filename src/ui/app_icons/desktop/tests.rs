use super::*;
#[test]
fn game_name_beats_a_generic_wine_binary_and_resolves_the_steam_icon() {
    let directory = env::temp_dir().join(format!("nova-desktop-icons-{}", std::process::id()));
    fs::create_dir_all(directory.join("applications")).unwrap();
    fs::create_dir_all(directory.join("hicolor/32x32/apps")).unwrap();
    fs::write(directory.join("applications/poe.desktop"), "[Desktop Entry]\nName=Path of Exile\nIcon=steam_icon_238960\nExec=steam steam://rungameid/238960").unwrap();
    let icon = directory.join("hicolor/32x32/apps/steam_icon_238960.png");
    fs::write(&icon, []).unwrap();
    let mut entries = vec![Entry {
        identities: vec!["wine64preloader".into()],
        icon: "wine".into(),
    }];
    scan(&directory.join("applications"), 0, &mut entries);
    let resolver = Resolver {
        entries,
        roots: vec![directory.clone()],
        theme: "hicolor".into(),
    };
    let query = Query {
        key: "wine64-preloader".into(),
        label: "PathOfExileSteam.exe".into(),
        binary: Some("wine64-preloader".into()),
        icon: None,
    };
    assert_eq!(resolver.find(&query), Some(icon));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn matches_wine_steam_names_and_keeps_desktop_actions_out() {
    assert_eq!(
        normalize("PathOfExileSteam.exe"),
        normalize("Path of Exile")
    );
    assert_eq!(normalize("/usr/bin/spotify"), normalize("Spotify"));
    assert_eq!(
        section_value(
            "[Desktop Entry]\nIcon=spotify\n[Desktop Action Play]\nIcon=wrong",
            "Desktop Entry",
            "Icon"
        ),
        Some("spotify".into())
    );
}
