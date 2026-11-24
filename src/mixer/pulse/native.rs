use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use libpulse_binding::{
    callbacks::ListResult,
    proplist::Proplist,
    volume::{ChannelVolumes, Volume},
};

use super::{Module, Sink, Stream, client::with_client};

fn properties(list: &Proplist) -> BTreeMap<String, String> {
    list.iter()
        .filter_map(|key| list.get_str(&key).map(|value| (key, value)))
        .collect()
}

fn collect<U, T>(
    result: ListResult<&U>,
    items: &mut Vec<T>,
    output: &RefCell<Option<Vec<T>>>,
    convert: impl FnOnce(&U) -> T,
) {
    match result {
        ListResult::Item(info) => items.push(convert(info)),
        ListResult::End => *output.borrow_mut() = Some(std::mem::take(items)),
        ListResult::Error => {}
    }
}

pub(in crate::mixer) fn sinks() -> Result<Vec<Sink>, String> {
    with_client(|client| {
        client.request("list sinks", |context, output| {
            let mut items = Vec::new();
            context.introspect().get_sink_info_list(move |item| {
                collect(item, &mut items, &output, |info| Sink {
                    index: info.index,
                    name: info.name.as_deref().unwrap_or_default().to_owned(),
                    properties: properties(&info.proplist),
                });
            })
        })
    })
}

pub(in crate::mixer) fn streams() -> Result<Vec<Stream>, String> {
    with_client(|client| {
        client.request("list playback streams", |context, output| {
            let mut items = Vec::new();
            context.introspect().get_sink_input_info_list(move |item| {
                collect(item, &mut items, &output, |info| Stream {
                    index: info.index,
                    sink: info.sink,
                    corked: info.corked,
                    mute: info.mute,
                    properties: properties(&info.proplist),
                });
            })
        })
    })
}

pub(in crate::mixer) fn modules() -> Result<Vec<Module>, String> {
    with_client(|client| {
        client.request("list modules", |context, output| {
            let mut items = Vec::new();
            context.introspect().get_module_info_list(move |item| {
                collect(item, &mut items, &output, |info| Module {
                    index: info.index,
                    name: info.name.as_deref().unwrap_or_default().to_owned(),
                    argument: info.argument.as_deref().unwrap_or_default().to_owned(),
                });
            })
        })
    })
}

pub(in crate::mixer) fn defaults() -> Result<(Option<String>, Option<String>), String> {
    with_client(|client| {
        client.request("read defaults", |context, output| {
            context.introspect().get_server_info(move |info| {
                *output.borrow_mut() = Some((
                    info.default_sink_name.as_deref().map(str::to_owned),
                    info.default_source_name.as_deref().map(str::to_owned),
                ));
            })
        })
    })
}

pub(in crate::mixer) fn set_default_sink(name: &str) -> Result<(), String> {
    with_client(|client| {
        client.success("set default sink", |context, callback| {
            context.set_default_sink(name, callback)
        })
    })
}

pub(in crate::mixer) fn set_default_source(name: &str) -> Result<(), String> {
    with_client(|client| {
        client.success("set default source", |context, callback| {
            context.set_default_source(name, callback)
        })
    })
}

pub(in crate::mixer) fn load_module(name: &str, arguments: &[&str]) -> Result<u32, String> {
    let arguments = arguments.join(" ");
    with_client(|client| {
        client.request("load module", |context, output| {
            context
                .introspect()
                .load_module(name, &arguments, move |index| {
                    if index != u32::MAX {
                        *output.borrow_mut() = Some(index);
                    }
                })
        })
    })
}

pub(in crate::mixer) fn unload_module(index: u32) -> Result<(), String> {
    with_client(|client| {
        client.success("unload module", |context, callback| {
            context.introspect().unload_module(index, callback)
        })
    })
}

pub(in crate::mixer) fn move_stream(index: u32, sink: &str) -> Result<(), String> {
    with_client(|client| {
        client.success("move playback stream", |context, callback| {
            context
                .introspect()
                .move_sink_input_by_name(index, sink, Some(callback))
        })
    })
}

pub(in crate::mixer) fn set_volume(name: &str, percent: u8) -> Result<(), String> {
    with_client(|client| {
        let channels = client.request(
            "read sink channels",
            |context, output: Rc<RefCell<Option<u8>>>| {
                context
                    .introspect()
                    .get_sink_info_by_name(name, move |item| {
                        if let ListResult::Item(info) = item {
                            *output.borrow_mut() = Some(info.volume.len());
                        }
                    })
            },
        )?;
        let mut volumes = ChannelVolumes::default();
        volumes.set(
            channels,
            Volume(Volume::NORMAL.0 * u32::from(percent) / 100),
        );
        client.success("set sink volume", |context, callback| {
            context
                .introspect()
                .set_sink_volume_by_name(name, &volumes, Some(callback))
        })
    })
}

pub(in crate::mixer) fn set_mute(name: &str, muted: bool) -> Result<(), String> {
    with_client(|client| {
        client.success("set sink mute", |context, callback| {
            context
                .introspect()
                .set_sink_mute_by_name(name, muted, Some(callback))
        })
    })
}
