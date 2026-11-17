# Nova Control

[Українська](README.uk.md)

A native Rust application for SteelSeries Arctis Nova 7 Wireless Gen 2 (USB dongle `1038:227e`). The Overview shows connection, battery, active EQ, and quick actions. The microphone section in Mixer and header shortcut control the hardware Sidetone levels Off, Low, Medium and High. Nova Control reads the current value automatically and applies slider changes on release. Use **Зберегти в гарнітурі** (Save to headset) or `nova-control sidetone save` to commit the current device settings.

## Install or update

Download the Linux archive for your architecture and extract it. `BUILD.txt` lists the minimum glibc version for that particular build. Run the installer with Python 3.9 or later from the extracted folder.

```sh
python3 install.py
```

Search for **Nova Control** in your application menu. Run the same command from a newer archive to update, then reopen the app if an older copy is still running. The installer copies the executable to `~/.local/bin/nova-control` and adds a per-user desktop entry. You can move or delete the extracted folder after installation.

## Build from source

Use Rust 1.95+, Python 3.11+, a C compiler and pkg-config. On Debian or Ubuntu, install `build-essential pkg-config libwayland-dev libxkbcommon-dev libegl1-mesa-dev libgl1-mesa-dev libx11-dev`. Run the installer from the source directory to build and install.

```sh
python3 install.py
```

Repeat after extracting a newer source archive or updating your checkout with `git pull --ff-only`. Cargo reuses the release build cache. To build without installing, run `cargo build --release --locked`. For a CLI-only build, add `--no-default-features`.

The desktop app uses Wayland or X11 with OpenGL. If the local user lacks access to the dongle, follow the steps in the Ukrainian guide to install the udev rule. [Udev setup](README.uk.md#доступ-до-донгла)

## Command line

```sh
~/.local/bin/nova-control detect
~/.local/bin/nova-control sidetone get
~/.local/bin/nova-control sidetone set 1
```

Levels `0` through `3` match the four slider positions. If the USB interface is busy, close the other headset manager and try again.

## Build downloads

```sh
python3 scripts/package.py
(cd dist && sha256sum -c SHA256SUMS)
```

For a Debian 12 build targeting glibc 2.36, use `docker build --target artifacts --output type=local,dest=dist .`. Both paths produce an installable Linux archive, a source archive and `SHA256SUMS`. The [GitHub Actions workflow](.github/workflows/linux.yml) builds downloadable artifacts and attaches them to a release when a matching `v<version>` tag is pushed.

`src/device/` holds the device protocol and USB transports. `src/ui/` holds the Engine, layout, slider and theme. `src/cli.rs` handles the terminal interface. Run `cargo test --locked` and `python3 -m unittest discover -s tests` for checks without opening a window.

[Verified Sidetone protocol](https://github.com/Sapd/HeadsetControl/pull/560) · [Linux USB interface roles](https://lists.openwall.net/linux-kernel/2026/08/03/2425)

## Hardware EQ

The **EQ** page reads the active curve and offers four local presets. Use `nova-control eq list|get|select|export|import` from the terminal. The importer accepts Nova Control JSON and the peaking-filter subset of Sonar parametric EQ JSON. The headset applies the curve directly and Nova Control verifies it by reading it back. [Firmware EQ format](https://github.com/loteran/Arctis-Sound-Manager/issues/146)

Drag filters on the EQ graph or enter precise frequency, gain and Q values. A/B applies the previous or current curve to the headset for comparison. Search the local presets by name or filter by favorites. Use the save button or `nova-control eq save` to store the current device settings.

## PipeWire mixer

Enable the mixer to create Game, Media, Chat, and any custom sinks you configure. Assign active applications with the channel menu or by dragging them to a channel. Levels, mute state, order, colors and routes are stored in the XDG configuration. Run the app in the window or system tray, or use `nova-control mixer run` in a terminal. Requires PipeWire's Pulse server and `pactl`.

Enable **Фізичне коліщатко ChatMix** (physical ChatMix dial) to balance any two channels, including Game and Media. Other channels keep their own levels. CLI commands include `nova-control mixer chatmix Game Media`, `nova-control mixer chatmix off` and `nova-control mixer dial`.

The microphone section in Mixer controls mic gain. The Device panel controls the inactivity timeout and mute LED brightness. These are also available under `nova-control settings`.

## Desktop integration

Nova Control uses the StatusNotifierItem system tray where available. The tray menu can switch between the four Sidetone levels and reopen the last visible panel. The Device panel lets you choose Ukrainian or English and toggle desktop notification types independently. Notifications require the Linux notify-send command.

Use **Assign global shortcut** in Device settings to register a Sidetone cycle shortcut. This requires a desktop backend for the XDG Global Shortcuts portal. The desktop may ask for approval or let you choose a key combination.

The mixer CLI also supports **channel list**, **channel add NAME**, **channel rename NAME NEW**, and **channel remove NAME**. Existing Game, Media and Chat routes carry forward when updating from 0.2.0.

## Navigation and mixer layout

Mouse buttons 4 and 5 navigate backward and forward through visited panels. The Mixer includes Sidetone and mic gain, full-width channel rows, and the application list below them. Drag a channel by its grip to reorder it, or use Move up / Move down in its menu. All channel orders, including built-in channels, are saved.

Hardware ChatMix samples are collected on a dedicated thread with a 25 ms target interval. The UI receives changed samples immediately and animates the position. Headset battery and charging status are queried every 350 ms, with a filling battery indicator while charging.
