mod channel;
mod chatmix;
mod config;
mod config_store;
mod dial;
mod pulse;
mod session;

use std::{
    env,
    fs::{self, OpenOptions},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use pulse::{Sink, Stream};
use session::{Session, on_nova_channel};

pub use channel::{Channel, ChannelIcon, default_color};
pub use chatmix::Pair;
pub use config::Config;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppStream {
    pub key: String,
    pub label: String,
    pub channel: Option<Channel>,
    pub playing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub config: Config,
    pub streams: Vec<AppStream>,
    pub dial: Option<[u8; 2]>,
}

fn streams(currents: &[Stream], config: &Config) -> Vec<AppStream> {
    let mut apps = std::collections::BTreeMap::new();
    for stream in currents {
        if let Some(key) = stream.app_key() {
            let app = apps.entry(key).or_insert_with(|| AppStream {
                key: key.into(),
                label: stream.label().into(),
                channel: config.routes.get(key).copied(),
                playing: false,
            });
            app.playing |= stream.playing();
        }
    }
    apps.into_values().collect()
}

pub fn snapshot() -> Result<Snapshot, String> {
    let config = Config::load()?;
    let inputs = pulse::json::<Vec<Stream>>("sink-inputs")?;
    Ok(Snapshot {
        streams: streams(&inputs, &config),
        config,
        dial: None,
    })
}

pub fn run(
    mut stop: impl FnMut() -> bool,
    mut update: impl FnMut(Result<Snapshot, String>),
) -> Result<(), String> {
    let lock_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            config_store::path()
                .ok()
                .and_then(|path| path.parent().map(PathBuf::from))
        })
        .ok_or_else(|| "Немає каталогу стану мікшера.".to_owned())?;
    fs::create_dir_all(&lock_dir).map_err(|error| error.to_string())?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_dir.join("nova-control-mixer.lock"))
        .map_err(|error| error.to_string())?;
    lock.try_lock().map_err(|error| {
        format!("Мікшер Nova Control уже запущено або він недоступний ({error}).")
    })?;
    let reader = dial::DialReader::start();
    let mut controller = Controller::default();
    let mut latest = None;
    let mut next_route = Instant::now();
    while !stop() {
        let started = Instant::now();
        if started >= next_route {
            match controller.tick() {
                Ok(snapshot) => {
                    latest = Some(snapshot.clone());
                    update(Ok(snapshot));
                }
                Err(error) => {
                    update(Err(error));
                }
            }
            next_route = Instant::now() + Duration::from_millis(350);
        }
        let enabled = controller.session.is_some() && controller.pair.is_some();
        reader
            .enabled
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
        if let Some(position) = reader.latest().filter(|_| enabled)
            && (controller.dial != Some(position) || controller.levels.is_none())
        {
            controller.dial = Some(position);
            match controller.apply_levels() {
                Ok(()) => {
                    if let Some(snapshot) = latest.as_mut() {
                        snapshot.dial = Some(position);
                        update(Ok(snapshot.clone()));
                    }
                }
                Err(error) => update(Err(error)),
            }
        }
        thread::sleep(Duration::from_millis(15).saturating_sub(started.elapsed()));
    }
    Ok(())
}

#[derive(Default)]
struct Controller {
    session: Option<Session>,
    levels: Option<std::collections::BTreeMap<Channel, (u8, bool)>>,
    config: Config,
    pair: Option<Pair>,
    dial: Option<[u8; 2]>,
}

impl Controller {
    fn tick(&mut self) -> Result<Snapshot, String> {
        let config = Config::load()?;
        let mut sinks = pulse::json::<Vec<Sink>>("sinks")?;
        let Some(output) = sinks.iter().find(|sink| sink.is_nova()).cloned() else {
            self.session = None;
            self.dial = None;
            return Err("Очікую аудіовихід Nova 7 Gen 2 у PipeWire.".into());
        };
        let channels = config.channels();
        let missing = channels
            .iter()
            .any(|channel| !sinks.iter().any(|sink| sink.name == channel.sink()));
        let topology_changed = self
            .config
            .channels()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            != channels.iter().copied().collect();
        if self
            .session
            .as_ref()
            .is_none_or(|session| session.output.index != output.index)
            || missing
            || topology_changed
        {
            drop(self.session.take());
            self.session = Some(Session::open(output, &channels)?);
            self.levels = None;
            sinks = pulse::json::<Vec<Sink>>("sinks")?;
        }
        self.config = config.clone();
        if self.pair != config.pair {
            self.pair = config.pair;
            self.dial = None;
        }
        self.apply_levels()?;
        let inputs = pulse::json::<Vec<Stream>>("sink-inputs")?;
        let session = self.session.as_mut().expect("session initialized");
        session.retain_streams(&inputs);
        for input in &inputs {
            if let Some(app) = input.app_key() {
                if let Some(&channel) = config.routes.get(app) {
                    session.route(input, &sinks, channel)?;
                } else if session.managed(input.index) && on_nova_channel(input, &sinks) {
                    session.unroute(input, &sinks)?;
                }
            }
        }
        Ok(Snapshot {
            streams: streams(&inputs, &config),
            config,
            dial: self.dial,
        })
    }
    fn apply_levels(&mut self) -> Result<(), String> {
        let levels: std::collections::BTreeMap<_, _> = self
            .config
            .channels()
            .into_iter()
            .map(|channel| {
                let factor = self
                    .pair
                    .zip(self.dial)
                    .map_or(100, |(pair, factors)| pair.factor(channel, factors));
                (
                    channel,
                    (
                        chatmix::volume(self.config.level(channel), factor),
                        self.config.is_muted(channel),
                    ),
                )
            })
            .collect();
        if self.levels.as_ref() != Some(&levels) {
            // An interrupted update invalidates the cache so the next tick reconciles the sinks.
            let previous = self.levels.take();
            for (&channel, &(level, muted)) in &levels {
                let last = previous.as_ref().and_then(|old| old.get(&channel)).copied();
                let sink = channel.sink();
                if last.is_none_or(|old| old.0 != level) {
                    pulse::call(&["set-sink-volume", &sink, &format!("{level}%")])?;
                }
                if last.is_none_or(|old| old.1 != muted) {
                    pulse::call(&["set-sink-mute", &sink, if muted { "1" } else { "0" }])?;
                }
            }
            self.levels = Some(levels);
        }
        Ok(())
    }
}
