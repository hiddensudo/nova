use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::{PRESETS, Profile, from_file};

pub struct Entry {
    pub key: String,
    pub title: String,
    pub profile: Result<Profile, String>,
}

pub fn directory() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|path| path.join("nova-control/presets"))
}

pub fn load() -> Vec<Entry> {
    let mut entries: Vec<_> = PRESETS
        .iter()
        .map(|&name| Entry {
            key: name.to_owned(),
            title: name.to_owned(),
            profile: Profile::preset(name),
        })
        .collect();
    if let Some(root) = directory() {
        let mut imported = Vec::new();
        scan(&root, &root, 0, &mut imported);
        imported.sort_by_cached_key(|entry| entry.title.to_lowercase());
        entries.extend(imported);
    }
    entries
}

fn scan(root: &Path, dir: &Path, depth: u8, entries: &mut Vec<Entry>) {
    let Ok(children) = fs::read_dir(dir) else {
        return;
    };
    for child in children.flatten() {
        let path = child.path();
        if child.file_type().is_ok_and(|kind| kind.is_dir()) && depth < 2 {
            scan(root, &path, depth + 1, entries);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            && child.file_type().is_ok_and(|kind| kind.is_file())
            && child.metadata().is_ok_and(|stat| stat.len() <= 256 * 1024)
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && let Ok(relative) = path.strip_prefix(root)
        {
            let profile = if stem.ends_with(" [Mic]") {
                Err("Microphone preset. This catalog applies playback EQ only.".into())
            } else {
                from_file(&path)
            };
            entries.push(Entry {
                key: relative.to_string_lossy().into_owned(),
                title: stem.to_owned(),
                profile,
            });
        }
    }
}

pub fn select(name: &str) -> Result<Profile, String> {
    if let Ok(preset) = Profile::preset(name) {
        return Ok(preset);
    }
    let mut matches = load().into_iter().filter(|entry| {
        entry.key.eq_ignore_ascii_case(name) || entry.title.eq_ignore_ascii_case(name)
    });
    let entry = matches
        .next()
        .ok_or_else(|| format!("Unknown preset: {name}."))?;
    if matches.next().is_some() {
        return Err(format!(
            "Multiple presets match {name}. Select by the path shown by eq list."
        ));
    }
    entry.profile
}
