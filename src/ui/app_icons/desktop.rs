use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::mixer::AppStream;

#[derive(Clone, Hash, PartialEq, Eq)]
pub(super) struct Query {
    pub key: String,
    label: String,
    icon: Option<String>,
    binary: Option<String>,
}

impl From<&AppStream> for Query {
    fn from(app: &AppStream) -> Self {
        Self {
            key: app.key.clone(),
            label: app.label.clone(),
            icon: app.icon_name.clone(),
            binary: app.binary.clone(),
        }
    }
}

struct Entry {
    identities: Vec<String>,
    icon: String,
}

pub(super) struct Resolver {
    entries: Vec<Entry>,
    roots: Vec<PathBuf>,
    theme: String,
}

impl Resolver {
    pub fn new() -> Self {
        let home = env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        let data_dirs =
            env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        let mut shares = vec![data_home, home.join(".local/share/flatpak/exports/share")];
        shares.extend(env::split_paths(&data_dirs).filter(|p| p.is_absolute()));
        shares.push(PathBuf::from("/var/lib/flatpak/exports/share"));
        let mut entries = Vec::new();
        for share in &shares {
            scan(&share.join("applications"), 0, &mut entries);
        }
        let mut roots = vec![home.join(".icons")];
        roots.extend(shares.iter().map(|p| p.join("icons")));
        roots.extend(shares.iter().map(|p| p.join("pixmaps")));
        let config = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        let theme = fs::read_to_string(config.join("kdeglobals"))
            .ok()
            .and_then(|s| section_value(&s, "Icons", "Theme"))
            .unwrap_or_else(|| "hicolor".into());
        Self {
            entries,
            roots,
            theme,
        }
    }

    pub fn find(&self, query: &Query) -> Option<PathBuf> {
        if let Some(icon) = &query.icon
            && let Some(path) = super::theme::find(icon, &self.theme, &self.roots)
        {
            return Some(path);
        }
        let identities: Vec<_> = [
            Some(query.label.as_str()),
            Some(query.key.as_str()),
            query.binary.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(normalize)
        .filter(|s| !s.is_empty())
        .collect();
        let matched = self
            .entries
            .iter()
            .filter_map(|entry| {
                let score = identities
                    .iter()
                    .position(|id| entry.identities.contains(id))?;
                Some((score, entry))
            })
            .min_by_key(|(score, _)| *score);
        if let Some((_, entry)) = matched
            && let Some(path) = super::theme::find(&entry.icon, &self.theme, &self.roots)
        {
            return Some(path);
        }
        for name in [query.binary.as_deref(), Some(query.key.as_str())]
            .into_iter()
            .flatten()
        {
            if let Some(path) = super::theme::find(name, &self.theme, &self.roots) {
                return Some(path);
            }
        }
        None
    }
}

fn normalize(value: &str) -> String {
    let value = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .to_lowercase();
    let value = value
        .strip_suffix(".desktop")
        .or_else(|| value.strip_suffix(".exe"))
        .unwrap_or(&value);
    let value = if value.len() > 5 {
        value.strip_suffix("steam").unwrap_or(value)
    } else {
        value
    };
    value.chars().filter(|c| c.is_alphanumeric()).collect()
}

fn scan(directory: &Path, depth: usize, entries: &mut Vec<Entry>) {
    let Ok(children) = fs::read_dir(directory) else {
        return;
    };
    for child in children.flatten().take(4096) {
        let path = child.path();
        if child.file_type().is_ok_and(|kind| kind.is_dir()) && depth < 2 {
            scan(&path, depth + 1, entries);
        } else if path.extension().is_some_and(|ext| ext == "desktop")
            && fs::metadata(&path).is_ok_and(|m| m.len() < 128 * 1024)
            && let Ok(text) = fs::read_to_string(&path)
            && let Some(icon) = section_value(&text, "Desktop Entry", "Icon")
        {
            if section_value(&text, "Desktop Entry", "Hidden").as_deref() == Some("true") {
                continue;
            }
            let mut identities = vec![normalize(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default(),
            )];
            for key in ["Name", "StartupWMClass", "TryExec", "Exec"] {
                if let Some(value) = section_value(&text, "Desktop Entry", key) {
                    let value = if matches!(key, "Exec" | "TryExec") {
                        value
                            .split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .trim_matches('"')
                    } else {
                        &value
                    };
                    identities.push(normalize(value));
                }
            }
            entries.push(Entry { identities, icon });
        }
    }
}

pub(super) fn section_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut active = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            active = line == format!("[{section}]");
        } else if active
            && !line.starts_with('#')
            && let Some((name, value)) = line.split_once('=')
            && name.trim() == key
        {
            return Some(value.trim().to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests;
