mod import;
mod wire;

use serde::{Deserialize, Serialize};

use super::with_device;
pub use import::{from_file, to_file};

pub const BAND_COUNT: usize = 10;
pub const PRESETS: [&str; 4] = ["Flat", "Bass Boost", "Focus", "Smiley"];

const FREQUENCIES: [u16; BAND_COUNT] = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Band {
    pub hz: u16,
    pub gain_tenths: i8,
    pub q_thousandths: u16,
    pub filter: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Profile {
    pub name: String,
    pub kind: u8,
    pub bands: [Band; BAND_COUNT],
}

impl Profile {
    pub fn preset(name: &str) -> Result<Self, String> {
        let mut profile = Self::flat();
        let (name, gains) = match name.to_ascii_lowercase().as_str() {
            "flat" => (PRESETS[0], [0; BAND_COUNT]),
            "bass" | "bass boost" => (PRESETS[1], [35, 50, 35, 15, 0, 0, 0, 0, 0, 0]),
            "focus" => (PRESETS[2], [-35, -30, -20, -10, 0, 10, 25, 20, 5, 0]),
            "smiley" => (PRESETS[3], [25, 30, 15, 0, -15, -20, -10, 5, 25, 30]),
            _ => return Err(format!("Unknown preset: {name}.")),
        };
        profile.name = name.to_owned();
        profile.kind = u8::from(name != "Flat");
        for (band, gain_tenths) in profile.bands.iter_mut().zip(gains) {
            band.gain_tenths = gain_tenths;
        }
        Ok(profile)
    }

    fn flat() -> Self {
        Self {
            name: "Flat".into(),
            kind: 0,
            bands: FREQUENCIES.map(|hz| Band {
                hz,
                gain_tenths: 0,
                q_thousandths: 1414,
                filter: 1,
            }),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 32
            || !self.name.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
            || self.kind > 1
        {
            return Err("The EQ name must contain 1–32 printable ASCII characters.".into());
        }
        for (i, band) in self.bands.iter().enumerate() {
            if band.filter != 1
                || !(20..=20_000).contains(&band.hz)
                || !(-120..=120).contains(&band.gain_tenths)
                || !(200..=10_000).contains(&band.q_thousandths)
            {
                return Err(format!(
                    "Band {} must be a peaking filter: 20–20000 Hz, −12 to +12 dB, Q 0.2–10.",
                    i + 1
                ));
            }
        }
        Ok(())
    }
}

pub fn current() -> Result<Profile, String> {
    with_device(wire::read)
}

pub fn apply(profile: &Profile) -> Result<Profile, String> {
    profile.validate()?;
    with_device(|port| {
        let previous = wire::read(port)?;
        if previous == *profile {
            return Ok(previous);
        }
        previous
            .validate()
            .map_err(|error| format!("Cannot restore the current profile: {error}"))?;
        match wire::write_verified(port, profile) {
            Ok(actual) => Ok(actual),
            Err(error) => match wire::write_verified(port, &previous) {
                Ok(_) => Err(format!("{error} Restored the previous profile.")),
                Err(restore) => Err(format!(
                    "{error} Could not verify profile recovery. {restore}"
                )),
            },
        }
    })
}

pub fn save(desired: &Profile) -> Result<Profile, String> {
    desired.validate()?;
    with_device(|port| {
        if wire::read(port)? != *desired {
            return Err(
                "The active curve changed. Apply the desired profile before saving.".into(),
            );
        }
        super::protocol::persist(port)?;
        let actual = wire::read(port)?;
        if actual == *desired {
            Ok(actual)
        } else {
            Err("The headset reported a different curve after saving.".into())
        }
    })
}
