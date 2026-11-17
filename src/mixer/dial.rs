use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Keep only the latest hardware sample so route enumeration cannot queue stale positions.
pub(super) struct DialReader {
    pub enabled: Arc<AtomicBool>,
    position: Arc<Mutex<Option<[u8; 2]>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DialReader {
    pub fn start() -> Self {
        let enabled = Arc::new(AtomicBool::new(false));
        let position = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (active, sample, stopped) = (enabled.clone(), position.clone(), stop.clone());
        let thread = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let start = Instant::now();
                if active.load(Ordering::Relaxed) {
                    if let Ok(Some(position)) = crate::device::chatmix::try_position() {
                        *sample.lock().expect("dial sample") = Some(position);
                    }
                } else {
                    *sample.lock().expect("dial sample") = None;
                }
                thread::sleep(Duration::from_millis(25).saturating_sub(start.elapsed()));
            }
        });
        Self {
            enabled,
            position,
            stop,
            thread: Some(thread),
        }
    }

    pub fn latest(&self) -> Option<[u8; 2]> {
        *self.position.lock().expect("dial sample")
    }
}

impl Drop for DialReader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
