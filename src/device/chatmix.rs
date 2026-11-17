use std::time::Duration;

use super::{HidReports, protocol::query, with_device};

const STATUS: u8 = 0xb0;
const DIAL_TIMEOUT: Duration = Duration::from_millis(200);

pub fn position() -> Result<[u8; 2], String> {
    with_device(|port| decode(&status(port, DIAL_TIMEOUT)?))
}

pub(crate) fn try_position() -> Result<Option<[u8; 2]>, String> {
    super::try_with_device(|port| decode(&status(port, Duration::from_millis(70))?))
}

pub(super) fn status(port: &mut dyn HidReports, timeout: Duration) -> Result<Vec<u8>, String> {
    query(port, &[STATUS, 0], timeout)
}

fn decode(report: &[u8]) -> Result<[u8; 2], String> {
    if let [STATUS, 3, _, _, left @ 0..=100, right @ 0..=100, ..] = report {
        return Ok([*left, *right]);
    }
    Err("Гарнітура не повернула дійсну позицію ChatMix.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_endpoints_and_center_from_wireless_status() {
        for factors in [[100, 0], [100, 99], [0, 100]] {
            assert_eq!(
                decode(&[STATUS, 3, 87, 3, factors[0], factors[1]]),
                Ok(factors)
            );
        }
        assert!(decode(&[STATUS, 2, 0, 3, 100, 100]).is_err());
        assert!(decode(&[STATUS, 3, 0, 3, 101, 100]).is_err());
        assert!(decode(&[STATUS, 3, 0]).is_err());
    }
}
