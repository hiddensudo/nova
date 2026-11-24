use std::path::Path;

use crate::device::eq;

pub(super) fn dispatch(args: &[&str]) -> Result<(), String> {
    let active = match args {
        ["list"] => {
            for entry in eq::catalog::load() {
                if entry.profile.is_ok() {
                    println!("{}", entry.key);
                } else {
                    println!("{} (unsupported)", entry.key);
                }
            }
            return Ok(());
        }
        ["get"] => {
            println!(
                "{}",
                serde_json::to_string_pretty(&eq::current()?).map_err(|e| e.to_string())?
            );
            return Ok(());
        }
        ["save"] => eq::save(&eq::current()?)?,
        ["export", path] => return eq::to_file(Path::new(path), &eq::current()?),
        ["select", name] => eq::apply(&eq::catalog::select(name)?)?,
        ["import", path] => eq::apply(&eq::from_file(Path::new(path))?)?,
        _ => {
            return Err(
                "nova-control eq list|get|select <name>|import <JSON>|export <JSON>|save".into(),
            );
        }
    };
    println!("{}", active.name);
    Ok(())
}
