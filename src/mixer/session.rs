use std::collections::{BTreeMap, BTreeSet};

use super::{
    Channel,
    pulse::{self, LOOPBACK_ID, Sink, Stream},
};

pub(super) struct Session {
    pub output: Sink,
    previous: BTreeMap<u32, String>,
    managed: BTreeSet<u32>,
    modules: Vec<u32>,
}

impl Session {
    pub fn open(output: Sink) -> Result<Self, String> {
        let default_sink = pulse::call(&["get-default-sink"]).ok();
        let default_source = pulse::call(&["get-default-source"]).ok();
        cleanup(&output.name, |_| true)?;
        let mut session = Self {
            output,
            previous: BTreeMap::new(),
            managed: BTreeSet::new(),
            modules: Vec::new(),
        };
        let created = session.create_channels();
        // Restore defaults only if PipeWire selected one of our new nodes.
        if let (Some(before), Ok(after)) = (default_sink, pulse::call(&["get-default-sink"]))
            && Channel::ALL.iter().any(|channel| channel.sink() == after)
        {
            let _ = pulse::call(&["set-default-sink", &before]);
        }
        if let (Some(before), Ok(after)) = (default_source, pulse::call(&["get-default-source"]))
            && Channel::ALL
                .iter()
                .any(|channel| format!("{}.monitor", channel.sink()) == after)
        {
            let _ = pulse::call(&["set-default-source", &before]);
        }
        created?;
        Ok(session)
    }

    fn create_channels(&mut self) -> Result<(), String> {
        for channel in Channel::ALL {
            self.load(&[
                "load-module",
                "module-null-sink",
                &format!("sink_name={}", channel.sink()),
                &format!(
                    "sink_properties=device.description='Nova {}' priority.session=0",
                    channel.name()
                ),
            ])?;
            let target = self.output.name.clone();
            self.load(&[
                "load-module",
                "module-loopback",
                &format!("source={}.monitor", channel.sink()),
                &format!("sink={target}"),
                "sink_dont_move=true",
                "source_dont_move=true",
                "latency_msec=30",
                &format!("sink_input_properties=application.id={LOOPBACK_ID}"),
                &format!("source_output_properties=application.id={LOOPBACK_ID}"),
            ])?;
        }
        Ok(())
    }

    fn load(&mut self, args: &[&str]) -> Result<(), String> {
        let index = pulse::call(args)?
            .parse::<u32>()
            .map_err(|error| error.to_string())?;
        self.modules.push(index);
        Ok(())
    }

    pub fn route(
        &mut self,
        stream: &Stream,
        sinks: &[Sink],
        channel: Channel,
    ) -> Result<(), String> {
        self.managed.insert(stream.index);
        let current = sinks.iter().find(|sink| sink.index == stream.sink);
        if let Some(current) = current {
            if current.name == channel.sink() {
                return Ok(());
            }
            if !Channel::ALL
                .iter()
                .any(|channel| channel.sink() == current.name)
            {
                self.previous.insert(stream.index, current.name.clone());
            }
        }
        pulse::call(&["move-sink-input", &stream.index.to_string(), channel.sink()]).map(|_| ())
    }

    pub fn managed(&self, stream: u32) -> bool {
        self.managed.contains(&stream)
    }

    pub fn retain_streams(&mut self, streams: &[Stream]) {
        self.managed
            .retain(|index| streams.iter().any(|s| s.index == *index));
        self.previous
            .retain(|index, _| self.managed.contains(index));
    }

    pub fn unroute(&mut self, stream: &Stream, sinks: &[Sink]) -> Result<(), String> {
        self.managed.remove(&stream.index);
        let target = self
            .previous
            .remove(&stream.index)
            .filter(|name| sinks.iter().any(|sink| sink.name == *name))
            .unwrap_or_else(|| self.output.name.clone());
        pulse::call(&["move-sink-input", &stream.index.to_string(), &target]).map(|_| ())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let sinks = pulse::json::<Vec<Sink>>("sinks").unwrap_or_default();
        let target = sinks
            .iter()
            .find(|sink| sink.name == self.output.name)
            .or_else(|| {
                sinks.iter().find(|sink| {
                    !Channel::ALL
                        .iter()
                        .any(|channel| channel.sink() == sink.name)
                })
            });
        if let Ok(streams) = pulse::json::<Vec<Stream>>("sink-inputs") {
            for stream in streams
                .iter()
                .filter(|stream| on_nova_channel(stream, &sinks))
            {
                let previous = self
                    .previous
                    .get(&stream.index)
                    .filter(|name| sinks.iter().any(|sink| sink.name == **name));
                if let Some(name) = previous.or_else(|| target.map(|sink| &sink.name)) {
                    let _ = pulse::call(&["move-sink-input", &stream.index.to_string(), name]);
                }
            }
        }
        if !self.modules.is_empty() {
            let _ = cleanup("", |index| self.modules.contains(&index));
        }
    }
}

pub(super) fn on_nova_channel(stream: &Stream, sinks: &[Sink]) -> bool {
    sinks.iter().any(|sink| {
        sink.index == stream.sink
            && Channel::ALL
                .iter()
                .any(|channel| channel.sink() == sink.name)
    })
}

fn cleanup(fallback: &str, include: impl Fn(u32) -> bool) -> Result<(), String> {
    let mut modules = pulse::modules()?;
    modules.retain(|module| include(module.index) && module.owned());
    if modules.is_empty() {
        return Ok(());
    }
    if !fallback.is_empty() {
        let sinks = pulse::json::<Vec<Sink>>("sinks")?;
        for stream in pulse::json::<Vec<Stream>>("sink-inputs")?
            .iter()
            .filter(|stream| on_nova_channel(stream, &sinks))
        {
            let _ = pulse::call(&["move-sink-input", &stream.index.to_string(), fallback]);
        }
    }
    modules.sort_by_key(|module| module.name != "module-loopback");
    let mut error = None;
    for module in modules {
        if let Err(failure) = pulse::call(&["unload-module", &module.index.to_string()]) {
            error = Some(failure);
        }
    }
    error.map_or(Ok(()), Err)
}
