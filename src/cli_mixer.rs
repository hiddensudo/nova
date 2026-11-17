use std::sync::atomic::{AtomicBool, Ordering};

use crate::mixer::{self, ChannelIcon, Config, Pair, default_color};

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

pub(super) fn dispatch(args: &[&str]) -> Result<(), String> {
    match args {
        ["run"] => {
            // The process owns these handlers during this foreground command.
            unsafe {
                libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
                libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
            }
            let mut previous = None;
            mixer::run(|| STOP.load(Ordering::Relaxed), |result| {
                let message = match result {
                    Ok(state) => format!("Nova mixer · {} застосунків", state.streams.len()),
                    Err(error) => error,
                };
                if previous.as_ref() != Some(&message) {
                    eprintln!("{message}");
                    previous = Some(message);
                }
            })
        }
        ["dial"] => {
            let [left, right] = crate::device::chatmix::position()?;
            println!("{left} {right}");
            Ok(())
        }
        ["chatmix", "off"] => Config::edit(|config| config.pair = None),
        ["chatmix", left, right] => Config::change(|config| {
            config.pair = Some(Pair::new(config.parse(left)?, config.parse(right)?)?);
            Ok(())
        }),
        ["list"] => {
            let state = mixer::snapshot()?;
            for app in state.streams {
                println!("{}\t{}\t{}", app.key, app.label, app.channel.map(|channel| state.config.name(channel)).unwrap_or("–"));
            }
            Ok(())
        }
        ["route", app, "auto"] => Config::edit(|config| { config.routes.remove(*app); }),
        ["route", app, channel] => Config::change(|config| {
            config.routes.insert((*app).into(), config.parse(channel)?);
            Ok(())
        }),
        ["volume", channel, percent] => {
            let percent = percent.parse::<u8>().map_err(|_| "Гучність має бути 0–100.")?;
            Config::change(|config| config.set_level(config.parse(channel)?, percent))
        }
        ["channel", "list"] => {
            let config = Config::load()?;
            for channel in config.channels() {
                println!("{}\t{}%", config.name(channel), config.level(channel));
            }
            Ok(())
        }
        ["channel", "add", name] => Config::change(|config| {
            config.add_channel(name, default_color(), ChannelIcon::default(), config.channels().len())?;
            Ok(())
        }),
        ["channel", "rename", channel, name] => Config::change(|config| config.rename_channel(config.parse(channel)?, name)),
        ["channel", "remove", name] => Config::change(|config| config.remove_channel(config.parse(name)?)),
        _ => Err("nova-control mixer run|list|route <app-key> <channel|auto>|volume <channel> <0–100>|chatmix <left> <right>|chatmix off|dial|channel list|channel add <name>|channel rename <channel> <name>|channel remove <channel>".into()),
    }
}
