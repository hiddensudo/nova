use std::process::ExitCode;

use crate::device::{self, Operation, Sidetone};
#[cfg(feature = "gui")]
use crate::ui;

const HELP: &str = "Nova Control — Arctis Nova 7 Wireless Gen 2\n\n\
    nova-control gui             відкрити вікно Engine\n\
    nova-control detect          знайти донгл без надсилання команд\n\
    nova-control sidetone get    зчитати поточний рівень\n\
    nova-control sidetone set <0|1|2|3>\n\n\
    nova-control sidetone save   зберегти поточний рівень у гарнітурі\n\n\
    nova-control eq list|get|select|import|export\n\n\
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
        ["eq", rest @ ..] => return crate::cli_eq::dispatch(rest),
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
    Err(format!("Цю збірку зібрано лише з CLI.\n\n{HELP}"))
}
