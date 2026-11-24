use std::{fs, path::Path};

use serde_json::Value;

use super::Profile;

pub fn from_file(path: &Path) -> Result<Profile, String> {
    let data = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let document: Value = serde_json::from_str(&data).map_err(|error| error.to_string())?;
    let profile = if let Some(eq) = document.get("parametricEQ") {
        let label = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Imported EQ");
        from_sonar(label, eq)?
    } else {
        serde_json::from_value(document).map_err(|error| error.to_string())?
    };
    profile.validate()?;
    Ok(profile)
}

pub fn to_file(path: &Path, profile: &Profile) -> Result<(), String> {
    let data = serde_json::to_string_pretty(profile).map_err(|error| error.to_string())?;
    fs::write(path, data).map_err(|error| format!("{}: {error}", path.display()))
}

fn from_sonar(label: &str, eq: &Value) -> Result<Profile, String> {
    let mut profile = Profile::flat();
    let label: String = label
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(32)
        .collect();
    profile.name = if label.trim().is_empty() {
        "Imported EQ".into()
    } else {
        label
    };
    profile.kind = 1;
    if eq.get("enabled").and_then(Value::as_bool) == Some(false) {
        return Ok(profile);
    }
    for (index, band) in profile.bands.iter_mut().enumerate() {
        let key = format!("filter{}", index + 1);
        let Some(filter) = eq.get(&key) else { continue };
        if filter.get("enabled").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        if filter.get("type").and_then(Value::as_str) != Some("peakingEQ") {
            return Err(format!("{key}: hardware import supports peaking filters."));
        }
        band.hz = number(filter, "frequency", 20.0, 20_000.0)?.round() as u16;
        band.gain_tenths = (number(filter, "gain", -12.0, 12.0)? * 10.0).round() as i8;
        band.q_thousandths = (number(filter, "qFactor", 0.2, 10.0)? * 1000.0).round() as u16;
    }
    Ok(profile)
}

fn number(object: &Value, key: &str, min: f64, max: f64) -> Result<f64, String> {
    object
        .get(key)
        .and_then(Value::as_f64)
        .filter(|v| (min..=max).contains(v))
        .ok_or_else(|| format!("{key}: expected a value from {min} to {max}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sonar_import_translates_units_and_leaves_disabled_filters_flat() {
        let eq = serde_json::json!({"enabled": true,
            "filter1": {"frequency":45.6,"gain":-6.8,"qFactor":0.707,"type":"peakingEQ"},
            "filter2": {"enabled":false,"type":"highShelf"}});
        let profile = from_sonar("Game", &eq).unwrap();
        assert_eq!(profile.bands[0].hz, 46);
        assert_eq!(profile.bands[0].gain_tenths, -68);
        assert_eq!(profile.bands[0].q_thousandths, 707);
        assert_eq!(profile.bands[1].gain_tenths, 0);
        assert!(
            from_sonar(
                "Unsupported",
                &serde_json::json!({"filter1":{"type":"highShelf"}})
            )
            .is_err()
        );
    }
}
