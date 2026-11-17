use std::{
    process::{Command, Stdio},
    thread,
};

use crate::device::settings::Settings;

use super::preferences::Preferences;

#[derive(Default)]
pub(super) struct Notifications {
    connected: bool,
    low: bool,
}

impl Notifications {
    pub fn observe(&mut self, state: Settings, prefs: &Preferences) {
        let low = state.connected
            && !state.charging.is_charging()
            && state.battery.is_some_and(|percent| percent <= 20);
        if low && !self.low && prefs.low_battery {
            send(
                prefs
                    .language
                    .text("Низький заряд гарнітури", "Headset battery low"),
            );
        }
        if !state.connected && self.connected && prefs.disconnected {
            send(
                prefs
                    .language
                    .text("Гарнітуру від’єднано", "Headset disconnected"),
            );
        }
        self.connected = state.connected;
        self.low = low;
    }

    pub fn error(&mut self, disconnected: bool, message: &str, prefs: &Preferences) {
        if disconnected {
            if self.connected && prefs.disconnected {
                send(
                    prefs
                        .language
                        .text("Донгл від’єднано", "Dongle disconnected"),
                );
            }
            self.connected = false;
            self.low = false;
        } else if prefs.errors {
            send(message);
        }
    }
}

fn send(message: &str) {
    let message = message.to_owned();
    thread::spawn(move || {
        let _ = Command::new("notify-send")
            .args([
                "--app-name=Nova Control",
                "--icon=audio-headphones",
                "--",
                "Nova Control",
                &message,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
}
