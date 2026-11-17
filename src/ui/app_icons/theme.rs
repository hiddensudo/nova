use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::desktop::section_value;

pub(super) fn find(name: &str, theme: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_owned());
    }
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return None;
    }
    let files = if path.extension().is_some() {
        vec![name.to_owned()]
    } else {
        vec![format!("{name}.png"), format!("{name}.svg")]
    };
    let mut visited = HashSet::new();
    for theme in [theme, "hicolor", "breeze", "Adwaita"] {
        if let Some(path) = themed(theme, &files, roots, &mut visited) {
            return Some(path);
        }
    }
    roots
        .iter()
        .flat_map(|root| files.iter().map(move |name| root.join(name)))
        .find(|p| p.is_file())
}

fn themed(
    theme: &str,
    files: &[String],
    roots: &[PathBuf],
    visited: &mut HashSet<String>,
) -> Option<PathBuf> {
    if theme.contains(['/', '\\'])
        || theme == "."
        || theme == ".."
        || visited.len() >= 24
        || !visited.insert(theme.to_owned())
    {
        return None;
    }
    let mut inherited = Vec::new();
    for root in roots {
        let base = root.join(theme);
        let text = fs::read_to_string(base.join("index.theme")).unwrap_or_default();
        let mut directories: Vec<_> = [
            "32x32/apps",
            "48x48/apps",
            "24x24/apps",
            "scalable/apps",
            "64x64/apps",
            "128x128/apps",
            "256x256/apps",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        for key in ["Directories", "ScaledDirectories"] {
            if let Some(value) = section_value(&text, "Icon Theme", key) {
                directories.extend(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                );
            }
        }
        for directory in directories {
            let relative = Path::new(&directory);
            if relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                continue;
            }
            for file in files {
                let path = base.join(relative).join(file);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        if let Some(value) = section_value(&text, "Icon Theme", "Inherits") {
            inherited.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
            );
        }
    }
    inherited
        .iter()
        .find_map(|parent| themed(parent, files, roots, visited))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_theme_inheritance_and_supports_absolute_steam_icons() {
        let root = std::env::temp_dir().join(format!("nova-icons-{}", std::process::id()));
        fs::create_dir_all(root.join("custom")).unwrap();
        fs::create_dir_all(root.join("hicolor/32x32/apps")).unwrap();
        fs::write(
            root.join("custom/index.theme"),
            "[Icon Theme]\nInherits=hicolor\n",
        )
        .unwrap();
        let icon = root.join("hicolor/32x32/apps/steam_icon_238960.png");
        fs::write(&icon, []).unwrap();
        assert_eq!(
            find("steam_icon_238960", "custom", std::slice::from_ref(&root)),
            Some(icon.clone())
        );
        assert_eq!(find(icon.to_str().unwrap(), "custom", &[]), Some(icon));
        assert!(find("../secret", "custom", std::slice::from_ref(&root)).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
