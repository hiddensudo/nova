mod cli;
mod cli_eq;
mod device;
#[cfg(feature = "gui")]
mod ui;

fn main() -> std::process::ExitCode {
    cli::run()
}
