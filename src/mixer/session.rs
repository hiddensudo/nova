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
    pub fn open(output: Sink, channels: &[Channel]) -> Result<Self, String> {
        let (default_sink, default_source) = pulse::defaults().unwrap_or_default();
        cleanup(&output.name, |_| true)?;
        let mut session = Self {
            output,
            previous: BTreeMap::new(),
            managed: BTreeSet::new(),
            modules: Vec::new(),
        };
        let created = session.create_channels(channels);
        let (new_sink, new_source) = pulse::defaults().unwrap_or_default();
        // Restore defaults only if PipeWire selected one of our new nodes.
        if let (Some(before), Some(after)) = (default_sink, new_sink)
            && Channel::owns_sink(&after)
        {
            let _ = pulse::set_default_sink(&before);
        }
        if let (Some(before), Some(after)) = (default_source, new_source)
            && after
                .strip_suffix(".monitor")
                .is_some_and(Channel::owns_sink)
        {
            let _ = pulse::set_default_source(&before);
        }
        created?;
        Ok(session)
    }

    fn create_channels(&mut self, channels: &[Channel]) -> Result<(), String> {
        for &channel in channels {
            self.load(
                "module-null-sink",
                &[
                    &format!("sink_name={}", channel.sink()),
                    &format!(
                        "sink_properties=device.description='Nova {}' priority.session=0",
                        channel.builtin_name()
                    ),
                ],
            )?;
            let target = self.output.name.clone();
            self.load(
                "module-loopback",
                &[
                    &format!("source={}.monitor", channel.sink()),
                    &format!("sink={target}"),
                    "sink_dont_move=true",
                    "source_dont_move=true",
                    "latency_msec=30",
                    &format!("sink_input_properties=application.id={LOOPBACK_ID}"),
                    &format!("source_output_properties=application.id={LOOPBACK_ID}"),
                ],
            )?;
        }
        Ok(())
    }

    fn load(&mut self, name: &str, args: &[&str]) -> Result<(), String> {
        let index = pulse::load_module(name, args)?;
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
            if !Channel::owns_sink(&current.name) {
                self.previous.insert(stream.index, current.name.clone());
            }
        }
        pulse::move_stream(stream.index, &channel.sink())
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
        let target = self
            .previous
            .get(&stream.index)
            .filter(|name| sinks.iter().any(|sink| sink.name == **name))
            .unwrap_or(&self.output.name);
        pulse::move_stream(stream.index, target)?;
        self.previous.remove(&stream.index);
        self.managed.remove(&stream.index);
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let sinks = pulse::sinks().unwrap_or_default();
        let target = sinks
            .iter()
            .find(|sink| sink.name == self.output.name)
            .or_else(|| sinks.iter().find(|sink| !Channel::owns_sink(&sink.name)));
        if let Ok(streams) = pulse::streams() {
            for stream in streams
                .iter()
                .filter(|stream| on_nova_channel(stream, &sinks))
            {
                let previous = self
                    .previous
                    .get(&stream.index)
                    .filter(|name| sinks.iter().any(|sink| sink.name == **name));
                if let Some(name) = previous.or_else(|| target.map(|sink| &sink.name)) {
                    let _ = pulse::move_stream(stream.index, name);
                }
            }
        }
        if !self.modules.is_empty() {
            let _ = cleanup("", |index| self.modules.contains(&index));
        }
    }
}

pub(super) fn on_nova_channel(stream: &Stream, sinks: &[Sink]) -> bool {
    sinks
        .iter()
        .any(|sink| sink.index == stream.sink && Channel::owns_sink(&sink.name))
}

fn cleanup(fallback: &str, include: impl Fn(u32) -> bool) -> Result<(), String> {
    let mut modules = pulse::modules()?;
    modules.retain(|module| include(module.index) && module.owned());
    if modules.is_empty() {
        return Ok(());
    }
    if !fallback.is_empty() {
        let sinks = pulse::sinks()?;
        for stream in pulse::streams()?
            .iter()
            .filter(|stream| on_nova_channel(stream, &sinks))
        {
            let _ = pulse::move_stream(stream.index, fallback);
        }
    }
    modules.sort_by_key(|module| module.name != "module-loopback");
    let mut error = None;
    for module in modules {
        if let Err(failure) = pulse::unload_module(module.index) {
            error = Some(failure);
        }
    }
    error.map_or(Ok(()), Err)
}
