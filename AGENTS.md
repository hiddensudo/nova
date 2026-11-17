# Nova Control

- Follow the source map in README.md. Keep CLI, UI state and layout, device protocol, and Linux transports in focused modules.
- Use `cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings` and `python3 -m unittest discover -s tests`.
- Keep generated archives, local binaries and hardware logs outside source control. Build with Cargo.lock and use the checked-in installer and packaging script.
- During development on the owner's desktop, verify without opening an app window. Use the CLI for device checks and restore the original Sidetone level after write tests.
- Rebuild and update the owner's installed copy after changing the app, using `python3 install.py` or the same installer with an explicitly built executable.
- The local Obsidian project `Nova 7 Linux` has the task board, device protocol notes and hardware verification history. Update the relevant note when changing a verified behavior.
