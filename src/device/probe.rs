use serde::Serialize;

use super::{Model, READ_TIMEOUT, chatmix, with_model};

#[derive(Serialize)]
struct Status {
    connected: bool,
    battery: Option<u8>,
    charging_code: u8,
    chatmix: Option<[u8; 2]>,
}

pub fn run(product: &str) -> Result<(), String> {
    let model = match product {
        "227e" => Model::Nova7Gen2,
        "229e" => Model::Nova7xGen2,
        _ => return Err("Usage: nova-control probe <227e|229e>".into()),
    };
    // Only the status query runs here. An experimental device can never
    // reach the Sidetone, EQ, settings or persist paths through this command.
    let status = with_model(model, |port| decode(&chatmix::status(port, READ_TIMEOUT)?))?;
    let report = serde_json::json!({
        "model": model.name(),
        "usb_id": format!("{:04x}:{:04x}", Model::VENDOR, model.product()),
        "status": status,
    });
    println!("{report:#}");
    Ok(())
}

fn decode(report: &[u8]) -> Result<Status, String> {
    let [0xb0, link, battery, charging_code, ..] = report else {
        return Err("The headset returned an invalid wireless status report.".into());
    };
    let connected = *link == 3;
    Ok(Status {
        connected,
        battery: (connected && *battery <= 100).then_some(*battery),
        charging_code: *charging_code,
        chatmix: chatmix::decode(report).ok(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_only_valid_status_values() {
        let status = decode(&[0xb0, 3, 72, 1, 100, 44]).unwrap();
        assert_eq!(status.battery, Some(72));
        assert_eq!(status.chatmix, Some([100, 44]));
        let status = decode(&[0xb0, 0, 90, 3, 100, 100]).unwrap();
        assert!(!status.connected);
        assert_eq!(status.battery, None);
        assert_eq!(status.chatmix, None);
        assert!(decode(&[0xb0, 3]).is_err());
    }
}
