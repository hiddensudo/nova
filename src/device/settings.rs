use std::{thread, time::Duration};

use serde::Serialize;

use super::{
    HidReports, chatmix,
    protocol::{READ_TIMEOUT, persist, query, send_bytes},
    with_device,
};

const AUDIO: u8 = 0x20;
const UX: u8 = 0xa0;
const COMMAND_GAP: Duration = Duration::from_millis(40);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Settings {
    pub connected: bool,
    pub battery: Option<u8>,
    pub charging: ChargeState,
    pub mic_volume: u8,
    pub sleep_minutes: u8,
    pub mute_led: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargeState {
    Unknown,
    Charging,
    Plugged,
    Discharging,
}

#[cfg(feature = "gui")]
impl ChargeState {
    pub fn is_charging(self) -> bool {
        matches!(self, Self::Charging | Self::Plugged)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Change {
    MicVolume(u8),
    SleepMinutes(u8),
    MuteLed(u8),
}

impl Change {
    fn command(self) -> Result<[u8; 2], String> {
        match self {
            Self::MicVolume(value @ 0..=7) => Ok([0x37, value]),
            Self::SleepMinutes(value @ 0..=90) => Ok([0xa3, value]),
            Self::MuteLed(value @ 0..=3) => Ok([0xae, value]),
            _ => Err("Допустимі значення: мікрофон 0–7, сон 0–90 хв, LED 0–3.".into()),
        }
    }

    fn matches(self, settings: Settings) -> bool {
        match self {
            Self::MicVolume(value) => value == settings.mic_volume,
            Self::SleepMinutes(value) => value == settings.sleep_minutes,
            Self::MuteLed(value) => value == settings.mute_led,
        }
    }
}

pub fn current() -> Result<Settings, String> {
    with_device(read)
}

pub fn apply(change: Change) -> Result<Settings, String> {
    let command = change.command()?;
    with_device(|port| {
        let current = read(port)?;
        if !current.connected {
            return Err("Увімкни гарнітуру та дочекайся з’єднання через 2,4 ГГц.".into());
        }
        if change.matches(current) {
            return Ok(current);
        }
        thread::sleep(COMMAND_GAP);
        send_bytes(port, &command)?;
        for _ in 0..3 {
            thread::sleep(Duration::from_millis(100));
            if let Ok(actual) = read(port)
                && actual.connected
                && change.matches(actual)
            {
                return Ok(actual);
            }
        }
        Err("Гарнітура не підтвердила зміну налаштувань.".into())
    })
}

pub fn save() -> Result<Settings, String> {
    with_device(|port| {
        if !read(port)?.connected {
            return Err("Гарнітура має бути підключена для збереження.".into());
        }
        persist(port)?;
        read(port)
    })
}

fn read(port: &mut dyn HidReports) -> Result<Settings, String> {
    let status = chatmix::status(port, READ_TIMEOUT)?;
    thread::sleep(COMMAND_GAP);
    let audio = query(port, &[AUDIO], READ_TIMEOUT)?;
    thread::sleep(COMMAND_GAP);
    let ux = query(port, &[UX], READ_TIMEOUT)?;
    decode(&status, &audio, &ux)
}

fn decode(status: &[u8], audio: &[u8], ux: &[u8]) -> Result<Settings, String> {
    let (
        [0xb0, link, battery, charging, ..],
        [AUDIO, mic @ 0..=7, ..],
        [UX, timer, led @ 0..=3, ..],
    ) = (status, audio, ux)
    else {
        return Err("Неповна або некоректна відповідь про налаштування Nova 7 Gen 2.".into());
    };
    let connected = *link == 3;
    Ok(Settings {
        connected,
        battery: (connected && *battery <= 100).then_some(*battery),
        charging: match (connected, charging) {
            (true, 1) => ChargeState::Charging,
            (true, 2) => ChargeState::Plugged,
            (true, 3) => ChargeState::Discharging,
            _ => ChargeState::Unknown,
        },
        mic_volume: *mic,
        sleep_minutes: *timer,
        mute_led: *led,
    })
}

#[cfg(test)]
mod tests;
