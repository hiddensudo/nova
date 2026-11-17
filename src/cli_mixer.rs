use std::sync::atomic::{AtomicBool, Ordering};

use crate::mixer::{self, Channel, Config, Pair};

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
        ["chatmix", left, right] => {
            let pair = Pair::new(Channel::parse(left)?, Channel::parse(right)?)?;
            Config::edit(|config| config.pair = Some(pair))
        }
        ["list"] => {
            for app in mixer::snapshot()?.streams {
                println!("{}\t{}\t{}", app.key, app.label, app.channel.map(Channel::name).unwrap_or("–"));
            }
            Ok(())
        }
        ["route", app, "auto"] => Config::edit(|config| { config.routes.remove(*app); }),
        ["route", app, channel] => {
            let channel = Channel::parse(channel)?;
            Config::edit(|config| { config.routes.insert((*app).into(), channel); })
        }
        ["volume", channel, percent] => {
            let channel = Channel::parse(channel)?;
            let percent = percent.parse::<u8>().ok().filter(|&v| v <= 100)
                .ok_or_else(|| "Гучність має бути 0–100.".to_owned())?;
            Config::edit(|config| config.levels[channel as usize] = percent)
        }
        _ => Err("nova-control mixer run|list|route <app-key> <Game|Media|Chat|auto>|volume <channel> <0–100>|chatmix <left> <right>|chatmix off|dial".into()),
    }
}
