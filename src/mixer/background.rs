//! Session mixer that survives closing the desktop window.

use std::{
    fs::{self, OpenOptions, TryLockError},
    io::ErrorKind,
    path::Path,
    thread,
    time::Duration,
};

use super::{RunMode, Snapshot, config_store, runtime_dir};

const STATE: &str = "nova-control-mixer-state.json";
type Status = Result<Snapshot, String>;

fn preference_path() -> Result<std::path::PathBuf, String> {
    Ok(config_store::path()?.with_file_name("mixer-enabled"))
}

pub fn enabled() -> Result<bool, String> {
    match fs::read(preference_path()?) {
        Ok(value) => match value.as_slice() {
            b"1\n" => Ok(true),
            b"0\n" => Ok(false),
            _ => Err("Invalid mixer-enabled setting.".into()),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(feature = "gui")]
pub(crate) fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = preference_path()?;
    fs::create_dir_all(path.parent().expect("mixer configuration directory"))
        .map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, if enabled { b"1\n" } else { b"0\n" })
        .and_then(|()| fs::rename(temporary, path))
        .map_err(|error| error.to_string())
}

pub fn run(stop: impl Fn() -> bool) -> Result<(), String> {
    if !enabled()? {
        return Ok(());
    }
    let dir = runtime_dir()?;
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("nova-control-mixer-background.lock"))
        .map_err(|error| error.to_string())?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Ok(()),
        Err(TryLockError::Error(error)) => return Err(error.to_string()),
    }

    let path = dir.join(STATE);
    let _ = fs::remove_file(&path);
    let mut previous = None;
    while !stop() {
        if enabled().unwrap_or(false) {
            let mut report = |status| {
                if previous.as_ref() != Some(&status) {
                    match publish(&path, &status) {
                        Ok(()) => previous = Some(status),
                        Err(error) => eprintln!("Nova mixer status: {error}"),
                    }
                }
            };
            if let Err(error) = super::run(RunMode::Remembered, &stop, &mut report) {
                report(Err(error));
            }
        } else if previous.take().is_some() {
            let _ = fs::remove_file(&path);
        }
        // Keep the session process available for a later enable, without opening
        // PipeWire or the headset while disabled.
        thread::sleep(Duration::from_millis(400));
    }
    let _ = fs::remove_file(&path);
    Ok(())
}

fn publish(path: &Path, status: &Status) -> Result<(), String> {
    let bytes = serde_json::to_vec(status).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes)
        .and_then(|()| fs::rename(temporary, path))
        .map_err(|error| error.to_string())
}

#[cfg(feature = "gui")]
pub(crate) fn status_path() -> Result<std::path::PathBuf, String> {
    Ok(runtime_dir()?.join(STATE))
}
