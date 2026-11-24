use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::device::settings::Settings;

use super::preferences::Preferences;

#[derive(Default)]
pub(super) struct Notifications {
    connected: bool,
    low: bool,
    error_gate: ErrorGate,
}

impl Notifications {
    pub fn observe(&mut self, state: Settings, prefs: &Preferences) {
        self.error_gate.recovered(state.connected, Instant::now());
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
        self.error_gate.healthy_since = None;
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
        } else if prefs.errors && self.error_gate.admit(message, Instant::now()) {
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
                "--icon=nova-control",
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

#[derive(Default)]
struct ErrorGate {
    incident: bool,
    last_sent: Option<Instant>,
    healthy_since: Option<Instant>,
}

impl ErrorGate {
    fn recovered(&mut self, connected: bool, now: Instant) {
        if !connected {
            self.healthy_since = None;
        } else {
            let started = *self.healthy_since.get_or_insert(now);
            if now.saturating_duration_since(started) >= Duration::from_secs(15) {
                self.incident = false;
            }
        }
    }

    fn admit(&mut self, message: &str, now: Instant) -> bool {
        self.healthy_since = None;
        let busy = message.contains("os error 16")
            || message.contains("Dongle busy")
            || message.contains("EBUSY");
        if busy
            || self.incident
            || self
                .last_sent
                .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_secs(60))
        {
            return false;
        }
        self.incident = true;
        self.last_sent = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contention_is_silent_and_intermittent_success_does_not_repeat_errors() {
        let mut gate = ErrorGate::default();
        let now = Instant::now();
        for index in 0..100 {
            assert!(!gate.admit(
                "Device or resource busy (os error 16)",
                now + Duration::from_secs(index)
            ));
        }
        assert!(gate.admit("USB timeout", now));
        for index in 1..100 {
            let time = now + Duration::from_secs(index);
            gate.recovered(true, time);
            assert!(!gate.admit("Another USB read failure", time));
        }
        gate.recovered(true, now + Duration::from_secs(100));
        gate.recovered(true, now + Duration::from_secs(116));
        assert!(gate.admit("USB timeout", now + Duration::from_secs(117)));
    }
}
