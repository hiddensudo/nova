mod config;
mod pulse;
mod session;

use std::{
    env,
    fs::{self, OpenOptions},
    path::PathBuf,
    thread,
    time::Duration,
};

use pulse::{Sink, Stream};
use session::{Session, on_nova_channel};

pub use config::{Channel, Config};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppStream {
    pub key: String,
    pub label: String,
    pub channel: Option<Channel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub config: Config,
    pub streams: Vec<AppStream>,
}

fn streams(currents: &[Stream], config: &Config) -> Vec<AppStream> {
    let mut apps = std::collections::BTreeMap::new();
    for stream in currents {
        if let Some(key) = stream.app_key() {
            apps.entry(key).or_insert_with(|| AppStream {
                key: key.into(),
                label: stream.label().into(),
                channel: config.routes.get(key).copied(),
            });
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
    })
}

pub fn run(
    mut stop: impl FnMut() -> bool,
    mut update: impl FnMut(Result<Snapshot, String>),
) -> Result<(), String> {
    let lock_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            config::path()
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
    let mut controller = Controller::default();
    while !stop() {
        update(controller.tick());
        thread::sleep(Duration::from_millis(600));
    }
    Ok(())
}

#[derive(Default)]
struct Controller {
    session: Option<Session>,
    levels: Option<[u8; 3]>,
}

impl Controller {
    fn tick(&mut self) -> Result<Snapshot, String> {
        let config = Config::load()?;
        let mut sinks = pulse::json::<Vec<Sink>>("sinks")?;
        let Some(output) = sinks.iter().find(|sink| sink.is_nova()).cloned() else {
            self.session = None;
            return Err("Очікую аудіовихід Nova 7 Gen 2 у PipeWire.".into());
        };
        let missing = Channel::ALL
            .iter()
            .any(|channel| !sinks.iter().any(|sink| sink.name == channel.sink()));
        if self
            .session
            .as_ref()
            .is_none_or(|session| session.output.index != output.index)
            || missing
        {
            drop(self.session.take());
            self.session = Some(Session::open(output)?);
            self.levels = None;
            sinks = pulse::json::<Vec<Sink>>("sinks")?;
        }
        if self.levels != Some(config.levels) {
            for (channel, percent) in Channel::ALL.into_iter().zip(config.levels) {
                pulse::call(&["set-sink-volume", channel.sink(), &format!("{percent}%")])?;
            }
            self.levels = Some(config.levels);
        }
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
        })
    }
}
