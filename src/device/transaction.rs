use std::{
    env,
    fs::{File, OpenOptions, TryLockError},
    path::PathBuf,
};

pub(super) struct TransactionGuard {
    _file: Option<File>,
}

/// Serialize Nova's hardware commands across the window, session mixer and CLI.
pub(super) fn acquire(wait: bool) -> Result<Option<TransactionGuard>, String> {
    let Some(dir) = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
    else {
        return Ok(Some(TransactionGuard { _file: None }));
    };
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("nova-control-hid.lock"))
        .map_err(|error| error.to_string())?;
    if wait {
        file.lock().map_err(|error| error.to_string())?;
    } else {
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(TryLockError::Error(error)) => return Err(error.to_string()),
        }
    }
    Ok(Some(TransactionGuard { _file: Some(file) }))
}
