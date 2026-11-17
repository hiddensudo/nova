use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixDatagram,
    },
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use eframe::egui::Context;

pub(super) struct Instance {
    _lock: File,
    socket: UnixDatagram,
    path: PathBuf,
    activation: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    listener: Option<JoinHandle<()>>,
}

impl Instance {
    pub fn acquire() -> io::Result<Option<Self>> {
        let uid = unsafe { libc::geteuid() };
        let parent = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(env::temp_dir);
        Self::in_directory(&parent.join(format!("nova-control-{uid}")))
    }

    fn in_directory(directory: &Path) -> io::Result<Option<Self>> {
        fs::create_dir_all(directory)?;
        let metadata = fs::symlink_metadata(directory)?;
        if !metadata.is_dir() || metadata.uid() != unsafe { libc::geteuid() } {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "Nova Control runtime directory is not owned by this user.",
            ));
        }
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(directory.join("gui.lock"))?;
        let path = directory.join("gui.sock");
        for _ in 0..40 {
            match lock.try_lock() {
                Ok(()) => {
                    match fs::remove_file(&path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == ErrorKind::NotFound => {}
                        Err(error) => return Err(error),
                    }
                    let socket = UnixDatagram::bind(&path)?;
                    socket.set_nonblocking(true)?;
                    return Ok(Some(Self {
                        _lock: lock,
                        socket,
                        path,
                        activation: Arc::new(AtomicBool::new(false)),
                        stop: Arc::new(AtomicBool::new(false)),
                        listener: None,
                    }));
                }
                Err(fs::TryLockError::WouldBlock) => {
                    let client = UnixDatagram::unbound()?;
                    client.set_nonblocking(true)?;
                    match client.send_to(b"activate", &path) {
                        Ok(_) => return Ok(None),
                        Err(error)
                            if matches!(
                                error.kind(),
                                ErrorKind::NotFound
                                    | ErrorKind::ConnectionRefused
                                    | ErrorKind::WouldBlock
                            ) =>
                        {
                            thread::sleep(Duration::from_millis(25));
                        }
                        Err(error) => return Err(error),
                    }
                }
                Err(fs::TryLockError::Error(error)) => return Err(error),
            }
        }
        Err(io::Error::new(
            ErrorKind::TimedOut,
            "The running Nova Control instance did not respond.",
        ))
    }

    pub fn listen(&mut self, ctx: &Context) -> io::Result<()> {
        let socket = self.socket.try_clone()?;
        let ctx = ctx.clone();
        let stop = self.stop.clone();
        let activation = self.activation.clone();
        self.listener = Some(thread::spawn(move || {
            let mut buffer = [0; 32];
            while !stop.load(Ordering::Relaxed) {
                match socket.recv(&mut buffer) {
                    Ok(size) if &buffer[..size] == b"activate" => {
                        activation.store(true, Ordering::Release);
                        ctx.request_repaint();
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20))
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        }));
        Ok(())
    }

    pub fn activated(&self) -> bool {
        self.activation.swap(false, Ordering::AcqRel)
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.listener.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_launch_activates_primary_and_stale_socket_is_recovered() {
        let directory = env::temp_dir().join(format!("nova-instance-test-{}", std::process::id()));
        let mut primary = Instance::in_directory(&directory).unwrap().unwrap();
        primary.listen(&Context::default()).unwrap();
        assert!(Instance::in_directory(&directory).unwrap().is_none());
        let mut activated = false;
        for _ in 0..50 {
            if primary.activated() {
                activated = true;
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(activated);
        drop(primary);
        let _stale = UnixDatagram::bind(directory.join("gui.sock")).unwrap();
        let restarted = Instance::in_directory(&directory).unwrap().unwrap();
        drop(restarted);
        fs::remove_dir_all(directory).unwrap();
    }
}
