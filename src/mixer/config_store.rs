use std::{
    env,
    fs::{self, OpenOptions},
    io::ErrorKind,
    path::PathBuf,
};

use super::Config;

impl Config {
    pub fn load() -> Result<Self, String> {
        match fs::read(path()?) {
            Ok(bytes) => {
                let config: Self = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("mixer.json: {error}"))?;
                config.validate()?;
                Ok(config)
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn change<T>(apply: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        let path = path()?;
        let dir = path.parent().expect("configuration directory");
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("mixer-config.lock"))
            .map_err(|error| error.to_string())?;
        lock.lock().map_err(|error| error.to_string())?;
        let mut config = Self::load()?;
        config.revision = config
            .revision
            .checked_add(1)
            .ok_or("Mixer revision overflow.")?;
        let result = apply(&mut config)?;
        config.validate()?;
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?;
        fs::write(&temporary, bytes)
            .and_then(|()| fs::rename(&temporary, path))
            .map_err(|error| error.to_string())?;
        Ok(result)
    }

    pub fn edit(apply: impl FnOnce(&mut Self)) -> Result<(), String> {
        Self::change(|config| {
            apply(config);
            Ok(())
        })
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
