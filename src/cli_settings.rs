use crate::device::settings::{self, Change};

pub(super) fn dispatch(args: &[&str]) -> Result<(), String> {
    let current = match args {
        ["get"] => settings::current()?,
        ["save"] => settings::save()?,
        ["mic", value] => settings::apply(Change::MicVolume(parse(value)?))?,
        ["sleep", value] => settings::apply(Change::SleepMinutes(parse(value)?))?,
        ["led", value] => settings::apply(Change::MuteLed(parse(value)?))?,
        _ => return Err("nova-control settings get|mic <0–7>|sleep <0–90>|led <0–3>|save".into()),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&current).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn parse(value: &str) -> Result<u8, String> {
    value.parse().map_err(|_| "Expected an integer.".into())
}
