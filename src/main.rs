mod cli;
mod cli_eq;
mod cli_mixer;
mod cli_settings;
mod device;
mod mixer;
#[cfg(feature = "gui")]
mod ui;

fn main() -> std::process::ExitCode {
    cli::run()
}
