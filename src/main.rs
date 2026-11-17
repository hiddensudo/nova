mod cli;
mod device;
#[cfg(feature = "gui")]
mod ui;

fn main() -> std::process::ExitCode {
    cli::run()
}
