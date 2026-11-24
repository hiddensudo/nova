use std::process::ExitCode;

use crate::device::{self, Operation, Sidetone};
#[cfg(feature = "gui")]
use crate::ui;

const HELP: &str = "Nova Control — Arctis Nova 7 Wireless Gen 2\n\n\
    nova-control gui             open the Nova Control window\n\
    nova-control detect          find the dongle without sending commands\n\
    nova-control probe [USB PID]  read battery and ChatMix for testing\n\
    nova-control sidetone get    read the current level\n\
    nova-control sidetone set <0|1|2|3>\n\n\
    nova-control sidetone save   save the current level to the headset\n\n\
    nova-control eq list|get|select|import|export|save\n\n\
    nova-control mixer run|list|route|volume|chatmix|dial\n\n\
    nova-control settings get|mic|sleep|led|save\n\n\
    0 Off · 1 Low · 2 Medium · 3 High";

pub fn run() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let string_args: Option<Vec<_>> = args.iter().map(|arg| arg.to_str()).collect();
    match string_args.as_deref().map(dispatch) {
        Some(Ok(())) => ExitCode::SUCCESS,
        Some(Err(error)) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("{HELP}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(args: &[&str]) -> Result<(), String> {
    let operation = match args {
        ["--help" | "-h" | "help"] => {
            println!("{HELP}");
            return Ok(());
        }
        ["--version" | "-V"] => {
            println!("nova-control {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        [] | ["gui"] => return launch_gui(),
        ["settings", rest @ ..] => return crate::cli_settings::dispatch(rest),
        ["mixer", rest @ ..] => return crate::cli_mixer::dispatch(rest),
        ["eq", rest @ ..] => return crate::cli_eq::dispatch(rest),
        ["probe"] => return device::probe::run(None),
        ["probe", product] => return device::probe::run(Some(product)),
        ["detect"] => return device::detect().map(|port| println!("{port}")),
        ["sidetone", "get"] => Operation::Read,
        ["sidetone", "save"] => Operation::Save,
        ["sidetone", "set", level] => Operation::Set(Sidetone::from_argument(level)?),
        _ => return Err(HELP.into()),
    };
    let level = device::sidetone(operation)?;
    println!("{} {}", level as u8, level.label());
    Ok(())
}

#[cfg(feature = "gui")]
fn launch_gui() -> Result<(), String> {
    ui::run().map_err(|err| err.to_string())
}

#[cfg(not(feature = "gui"))]
fn launch_gui() -> Result<(), String> {
    Err(format!("This build includes the CLI only.\n\n{HELP}"))
}
