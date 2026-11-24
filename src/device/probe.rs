use serde::Serialize;

use super::{Model, READ_TIMEOUT, UsbPort, chatmix, model::StatusFormat, with_model};

#[derive(Serialize)]
struct Status {
    connected: bool,
    battery: Option<u8>,
    charging_code: u8,
    chatmix: Option<[u8; 2]>,
}

pub fn run(product: Option<&str>) -> Result<(), String> {
    let model = match product {
        Some(product) => explicit_model(product)?,
        None => match UsbPort::probe_models()?.as_slice() {
            [model] => *model,
            [] => return Err("No supported Nova 7 USB dongle was found.".into()),
            _ => {
                return Err(
                    "Multiple supported dongles found. Run nova-control probe with a USB product ID from lsusb.".into(),
                );
            }
        },
    };
    // Only the status query runs here. An experimental device can never
    // reach the Sidetone, EQ, settings or persist paths through this command.
    let status = with_model(model, |port| read_status(model, port))?;
    let report = serde_json::json!({
        "model": model.name(),
        "usb_id": format!("{:04x}:{:04x}", Model::VENDOR, model.product()),
        "status": status,
    });
    println!("{report:#}");
    Ok(())
}

fn explicit_model(product: &str) -> Result<Model, String> {
    let product = product.strip_prefix("1038:").unwrap_or(product);
    if product.len() == 4
        && let Ok(pid) = u16::from_str_radix(product, 16)
        && let Some(model) = Model::for_product(Model::VENDOR, pid)
    {
        return Ok(model);
    }
    let supported = Model::PROBE_MODELS.map(|model| format!("{:04x}", model.product()));
    Err(format!(
        "Unknown Nova 7 product ID {product}. Supported IDs: {}.",
        supported.join(", ")
    ))
}

fn read_status(model: Model, port: &mut dyn super::HidReports) -> Result<Status, String> {
    decode(model.status_format, &chatmix::status(port, READ_TIMEOUT)?)
}

fn decode(format: StatusFormat, report: &[u8]) -> Result<Status, String> {
    let [0xb0, link, battery, charging_code, ..] = report else {
        return Err("The headset returned an invalid wireless status report.".into());
    };
    let connected = match format {
        StatusFormat::Gen2 => *link == 3,
        StatusFormat::Gen1Percent | StatusFormat::Gen1Steps => *charging_code != 0,
    };
    let battery = match (format, *battery) {
        (StatusFormat::Gen1Steps, steps @ 0..=4) => Some(steps * 25),
        (StatusFormat::Gen2 | StatusFormat::Gen1Percent, percent @ 0..=100) => Some(percent),
        _ => None,
    };
    Ok(Status {
        connected,
        battery: battery.filter(|_| connected),
        charging_code: *charging_code,
        chatmix: chatmix::factors(report).filter(|_| connected),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_only_valid_status_values() {
        let status = decode(StatusFormat::Gen2, &[0xb0, 3, 72, 1, 100, 44]).unwrap();
        assert_eq!(status.battery, Some(72));
        assert_eq!(status.chatmix, Some([100, 44]));
        let status = decode(StatusFormat::Gen2, &[0xb0, 0, 90, 3, 100, 100]).unwrap();
        assert!(!status.connected);
        assert_eq!(status.battery, None);
        assert_eq!(status.chatmix, None);
        assert!(decode(StatusFormat::Gen2, &[0xb0, 3]).is_err());
    }

    #[test]
    fn first_generation_uses_the_battery_format_for_its_usb_id() {
        let report = [0xb0, 1, 3, 2, 100, 44];
        for product in ["2202", "2206", "22A4"] {
            let model = explicit_model(product).unwrap();
            let status = decode(model.status_format, &report).unwrap();
            assert!(status.connected);
            assert_eq!(status.battery, Some(75));
            assert_eq!(status.chatmix, Some([100, 44]));
            assert_eq!(
                decode(model.status_format, &[0xb0, 1, 5, 2])
                    .unwrap()
                    .battery,
                None
            );
        }
        for product in ["1038:22A1", "22a5"] {
            let model = explicit_model(product).unwrap();
            let status = decode(model.status_format, &report).unwrap();
            assert_eq!(status.battery, Some(3));
            let offline = decode(model.status_format, &[0xb0, 3, 75, 0, 100, 100]).unwrap();
            assert!(!offline.connected);
            assert_eq!(offline.battery, None);
            assert_eq!(offline.chatmix, None);
        }
        assert!(explicit_model("220a").is_err());
    }

    #[test]
    fn simulated_dongles_receive_only_the_status_query() {
        use super::super::{HidReports, REPORT_SIZE};
        use std::{io, time::Duration};

        struct SimulatedDongle {
            packet: [u8; 6],
            requested: bool,
        }

        impl HidReports for SimulatedDongle {
            fn write_report(&mut self, command: &[u8; REPORT_SIZE]) -> io::Result<()> {
                assert!(!self.requested);
                assert_eq!(&command[..3], &[0, 0xb0, 0]);
                self.requested = true;
                Ok(())
            }

            fn read_report(
                &mut self,
                response: &mut [u8; REPORT_SIZE],
                _: Duration,
            ) -> io::Result<usize> {
                assert!(self.requested);
                response[..6].copy_from_slice(&self.packet);
                Ok(6)
            }
        }

        for (product, raw_battery, percent) in [("229e", 67, 67), ("2206", 3, 75), ("22a5", 67, 67)]
        {
            for position in [[0, 100], [100, 100], [100, 0]] {
                let mut dongle = SimulatedDongle {
                    packet: [0xb0, 3, raw_battery, 3, position[0], position[1]],
                    requested: false,
                };
                let model = explicit_model(product).unwrap();
                let state = read_status(model, &mut dongle).unwrap();
                assert_eq!(state.battery, Some(percent));
                assert_eq!(state.chatmix, Some(position));
            }
        }
    }
}
