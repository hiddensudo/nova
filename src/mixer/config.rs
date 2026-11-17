use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::ErrorKind,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use super::Pair;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Game,
    Media,
    Chat,
}

impl Channel {
    pub const ALL: [Self; 3] = [Self::Game, Self::Media, Self::Chat];

    pub fn name(self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Media => "Media",
            Self::Chat => "Chat",
        }
    }

    pub(super) fn sink(self) -> &'static str {
        match self {
            Self::Game => "nova_control_game",
            Self::Media => "nova_control_media",
            Self::Chat => "nova_control_chat",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|channel| channel.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| "Канал має бути Game, Media або Chat.".into())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Config {
    pub routes: BTreeMap<String, Channel>,
    pub levels: [u8; 3],
    pub pair: Option<Pair>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            routes: BTreeMap::new(),
            levels: [100; 3],
            pair: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        match fs::read(path()?) {
            Ok(bytes) => {
                let config: Self = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("mixer.json: {error}"))?;
                if config.levels.iter().any(|&level| level > 100) {
                    return Err("mixer.json: гучність каналу має бути 0–100.".into());
                }
                if let Some(pair) = config.pair {
                    Pair::new(pair.left, pair.right)?;
                }
                Ok(config)
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn edit(apply: impl FnOnce(&mut Self)) -> Result<(), String> {
        let path = path()?;
        let dir = path.parent().expect("config directory");
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let lock =
            File::create(dir.join("mixer-config.lock")).map_err(|error| error.to_string())?;
        lock.lock().map_err(|error| error.to_string())?;
        let mut config = Self::load()?;
        apply(&mut config);
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?;
        fs::write(&temporary, bytes)
            .and_then(|()| fs::rename(&temporary, path))
            .map_err(|error| error.to_string())
    }
}

pub(super) fn path() -> Result<PathBuf, String> {
    let root = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| "Немає каталогу конфігурації.".to_owned())?;
    Ok(root.join("nova-control/mixer.json"))
}
