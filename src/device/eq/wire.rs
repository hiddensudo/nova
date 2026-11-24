use std::{thread, time::Duration};

use super::{BAND_COUNT, Band, Profile};
use crate::device::{
    HidReports,
    protocol::{READ_TIMEOUT, query, send_bytes},
};

const READ_BANDS: u8 = 0x32;
const READ_NAME: u8 = 0xa6;
const WRITE_NAME: u8 = 0xa7;
const WRITE_BANDS: u8 = 0x33;
const APPLY: u8 = 0x27;
const WIRELESS: u8 = 0;
const BAND_BYTES: usize = 6;
const COMMAND_GAP: Duration = Duration::from_millis(40);

pub(super) fn read(port: &mut dyn HidReports) -> Result<Profile, String> {
    let bands = query(port, &[READ_BANDS, WIRELESS], READ_TIMEOUT)?;
    thread::sleep(COMMAND_GAP);
    let name = query(port, &[READ_NAME, WIRELESS], READ_TIMEOUT)?;
    decode(&bands, &name)
}

fn decode(bands: &[u8], name: &[u8]) -> Result<Profile, String> {
    if !bands.starts_with(&[READ_BANDS, WIRELESS])
        || !name.starts_with(&[READ_NAME, WIRELESS])
        || bands.len() < 2 + BAND_COUNT * BAND_BYTES
        || name.len() < 4
    {
        return Err("Invalid hardware EQ response.".into());
    }
    let name_bytes = name[3..].split(|&b| b == 0).next().unwrap_or_default();
    let label = std::str::from_utf8(name_bytes)
        .map_err(|_| "The headset returned an invalid EQ name.".to_owned())?;
    if label.is_empty() {
        return Err("The headset returned an empty EQ name.".into());
    }
    let bands = std::array::from_fn(|i| {
        let b = &bands[2 + i * BAND_BYTES..];
        Band {
            hz: u16::from_le_bytes([b[0], b[1]]),
            filter: b[2],
            gain_tenths: b[3] as i8,
            q_thousandths: u16::from_le_bytes([b[4], b[5]]),
        }
    });
    Ok(Profile {
        name: label.into(),
        kind: name[2],
        bands,
    })
}

fn frames(profile: &Profile) -> [Vec<u8>; 3] {
    let mut name = vec![WRITE_NAME, WIRELESS, profile.kind];
    name.extend(profile.name.bytes());
    let mut bands = vec![WRITE_BANDS, WIRELESS];
    for band in profile.bands {
        bands.extend(band.hz.to_le_bytes());
        bands.extend([band.filter, band.gain_tenths as u8]);
        bands.extend(band.q_thousandths.to_le_bytes());
    }
    [name, bands, vec![APPLY]]
}

pub(super) fn write_verified(
    port: &mut dyn HidReports,
    profile: &Profile,
) -> Result<Profile, String> {
    for body in frames(profile) {
        thread::sleep(COMMAND_GAP);
        send_bytes(port, &body)?;
    }
    for _ in 0..3 {
        thread::sleep(Duration::from_millis(110));
        if let Ok(actual) = read(port)
            && *profile == actual
        {
            return Ok(actual);
        }
    }
    Err("The headset did not confirm the new EQ name and curve.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_keep_connection_offset_and_little_endian_fields() {
        let mut profile = Profile::preset("flat").unwrap();
        profile.bands[0].gain_tenths = -15;
        let [name, bands, apply] = frames(&profile);
        assert_eq!(&name, &[0xa7, 0, 0, b'F', b'l', b'a', b't']);
        assert_eq!(&bands[..8], &[0x33, 0, 32, 0, 1, 241, 0x86, 5]);
        assert_eq!(apply, [0x27]);
        let mut bands = bands;
        let mut name = name;
        bands[0] = READ_BANDS;
        name[0] = READ_NAME;
        assert_eq!(decode(&bands, &name).unwrap(), profile);
        assert!(decode(&bands[..61], &name).is_err());
    }
}
