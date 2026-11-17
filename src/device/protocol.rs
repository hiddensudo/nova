use std::{
    io::ErrorKind,
    thread,
    time::{Duration, Instant},
};

use super::{HidReports, REPORT_SIZE};

const SETTINGS: u8 = 0x20;
const SET_SIDETONE: u8 = 0x39;
const SAVE: u8 = 0x09;
const SAVE_SETTLE: Duration = Duration::from_millis(250);
pub(super) const READ_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Sidetone {
    Off = 0,
    Low,
    Medium,
    High,
}

impl Sidetone {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }

    pub fn from_argument(value: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|level| {
                value == (*level as u8).to_string() || value.eq_ignore_ascii_case(level.label())
            })
            .ok_or_else(|| "Вкажи 0 (Off), 1 (Low), 2 (Medium) або 3 (High).".into())
    }
}

#[derive(Clone, Copy)]
pub enum Operation {
    Read,
    Set(Sidetone),
    Save,
}

pub(super) fn transact(
    file: &mut dyn HidReports,
    operation: Operation,
    timeout: Duration,
) -> Result<Sidetone, String> {
    match operation {
        Operation::Read => read_level(file, timeout),
        Operation::Save => {
            let expected = read_level(file, timeout)?;
            persist(file)?;
            let actual = read_level(file, timeout)?;
            if actual != expected {
                return Err("Рівень Sidetone змінився під час збереження.".into());
            }
            Ok(actual)
        }
        Operation::Set(desired) => {
            // Check that this collection answers the verified settings query before writing.
            read_level(file, timeout)?;
            send(file, SET_SIDETONE, desired as u8)?;
            // Allow the wireless headset to apply the setting before requesting it back.
            thread::sleep(Duration::from_millis(100));
            let mut confirmed = read_level(file, timeout);
            for _ in 0..2 {
                if matches!(confirmed, Ok(level) if level == desired) {
                    break;
                }
                thread::sleep(Duration::from_millis(75));
                confirmed = read_level(file, timeout);
            }
            match confirmed {
                Ok(level) if level == desired => Ok(level),
                Ok(level) => Err(format!(
                    "Гарнітура повернула {} після запиту {}.",
                    level.label(),
                    desired.label()
                )),
                Err(error) => Err(format!(
                    "Команду надіслано, новий рівень не підтверджено. {error}"
                )),
            }
        }
    }
}

pub(super) fn persist(port: &mut dyn HidReports) -> Result<(), String> {
    thread::sleep(SAVE_SETTLE);
    send(port, SAVE, 0)?;
    thread::sleep(SAVE_SETTLE);
    Ok(())
}

fn send(port: &mut dyn HidReports, command: u8, value: u8) -> Result<(), String> {
    send_bytes(port, &[command, value])
}

pub(super) fn send_bytes(port: &mut dyn HidReports, body: &[u8]) -> Result<(), String> {
    let mut report = [0; REPORT_SIZE];
    let command = body[0];
    report[1..1 + body.len()].copy_from_slice(body);
    loop {
        match port.write_report(&report) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("Запис HID 0x{command:02x} не вдався ({error}).")),
        }
    }
}

fn read_level(port: &mut dyn HidReports, timeout: Duration) -> Result<Sidetone, String> {
    let reply = query(port, &[SETTINGS, 0], timeout)?;
    decode_settings(&reply)?.ok_or_else(|| "Неочікувана відповідь Sidetone.".into())
}

pub(super) fn query(
    port: &mut dyn HidReports,
    body: &[u8],
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    send_bytes(port, body)?;
    let deadline = Instant::now() + timeout;
    let mut reply = [0; REPORT_SIZE];

    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        match port.read_report(&mut reply, remaining) {
            Ok(0) => return Err("З’єднання з донглом перервано.".into()),
            Ok(size) if reply[0] == body[0] => return Ok(reply[..size].to_vec()),
            Ok(_) => {}
            Err(error)
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
            Err(error) if error.kind() == ErrorKind::TimedOut => break,
            Err(error) => return Err(format!("Читання HID не вдалося ({error}).")),
        }
    }
    Err(format!(
        "Донгл не відповів на запит 0x{:02x}. Перевір з’єднання через 2,4 ГГц.",
        body[0]
    ))
}

fn decode_settings(reply: &[u8]) -> Result<Option<Sidetone>, String> {
    if reply.first() != Some(&SETTINGS) {
        return Ok(None);
    }
    if reply.len() < 4 {
        return Err("Закоротка відповідь на запит Sidetone.".into());
    }
    let value = reply[2];
    Sidetone::ALL
        .get(usize::from(value))
        .copied()
        .map(Some)
        .ok_or_else(|| format!("Невідомий апаратний рівень Sidetone ({value})."))
}

#[cfg(test)]
mod tests;
