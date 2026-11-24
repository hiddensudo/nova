use std::{collections::BTreeSet, env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum Language {
    Ukrainian,
    #[default]
    English,
}

impl Language {
    pub fn text(self, ukrainian: &'static str, english: &'static str) -> &'static str {
        match self {
            Self::Ukrainian => ukrainian,
            Self::English => english,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub language: Language,
    pub low_battery: bool,
    pub disconnected: bool,
    pub errors: bool,
    pub global_shortcut: bool,
    pub favorites: BTreeSet<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: Language::default(),
            low_battery: true,
            disconnected: true,
            errors: true,
            global_shortcut: false,
            favorites: BTreeSet::new(),
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        path()
            .ok()
            .and_then(|path| fs::read(path).ok())
            .and_then(|content| serde_json::from_slice(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = path()?;
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let data = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        fs::create_dir_all(path.parent().expect("configuration directory"))
            .and_then(|()| fs::write(&temporary, data))
            .and_then(|()| fs::rename(&temporary, &path))
            .map_err(|error| error.to_string())
    }
}

fn path() -> Result<PathBuf, String> {
    let directory = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| "Nova Control configuration directory is unavailable.".to_owned())?;
    Ok(directory.join("nova-control/ui.json"))
}
