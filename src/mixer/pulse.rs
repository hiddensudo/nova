use std::{
    collections::BTreeMap,
    process::{Command, Stdio},
};

use serde::{Deserialize, de::DeserializeOwned};

use super::Channel;

pub(super) const LOOPBACK_ID: &str = "nova-control.loopback";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct Sink {
    pub index: u32,
    pub name: String,
    #[serde(default)]
    properties: BTreeMap<String, String>,
}

impl Sink {
    pub fn is_nova(&self) -> bool {
        self.properties
            .get("device.vendor.id")
            .is_some_and(|v| v == "0x1038")
            && self
                .properties
                .get("device.product.id")
                .is_some_and(|v| v.eq_ignore_ascii_case("0x227e"))
            && self
                .properties
                .get("device.bus")
                .is_some_and(|v| v == "usb")
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Stream {
    pub index: u32,
    pub sink: u32,
    #[serde(default)]
    corked: bool,
    #[serde(default)]
    mute: bool,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

impl Stream {
    pub fn playing(&self) -> bool {
        !self.corked && !self.mute
    }

    pub fn app_key(&self) -> Option<&str> {
        if self
            .properties
            .get("application.id")
            .is_some_and(|value| value == LOOPBACK_ID)
        {
            return None;
        }
        [
            "application.id",
            "application.process.binary",
            "application.name",
            "node.name",
        ]
        .into_iter()
        .find_map(|key| {
            self.properties
                .get(key)
                .filter(|v| !v.is_empty())
                .map(String::as_str)
        })
    }

    pub fn label(&self) -> &str {
        self.properties
            .get("application.name")
            .map(String::as_str)
            .or_else(|| self.app_key())
            .unwrap_or("Audio stream")
    }
}

pub(super) struct Module {
    pub index: u32,
    pub name: String,
    pub argument: String,
}

impl Module {
    pub fn owned(&self) -> bool {
        let args = &self.argument;
        match self.name.as_str() {
            "module-null-sink" => args
                .split_whitespace()
                .filter_map(|field| field.strip_prefix("sink_name="))
                .any(Channel::owns_sink),
            "module-loopback" => {
                args.contains(LOOPBACK_ID)
                    && args
                        .split_whitespace()
                        .filter_map(|field| field.strip_prefix("source=")?.strip_suffix(".monitor"))
                        .any(Channel::owns_sink)
            }
            _ => false,
        }
    }
}

pub(super) fn modules() -> Result<Vec<Module>, String> {
    Ok(call(&["list", "short", "modules"])?
        .lines()
        .filter_map(|line| {
            let mut columns = line.splitn(4, '\t');
            Some(Module {
                index: columns.next()?.parse().ok()?,
                name: columns.next()?.into(),
                argument: columns.next().unwrap_or_default().into(),
            })
        })
        .collect())
}

pub(super) fn json<T: DeserializeOwned>(kind: &str) -> Result<T, String> {
    serde_json::from_str(&call(&["-f", "json", "list", kind])?)
        .map_err(|error| format!("pactl {kind}: {error}"))
}

pub(super) fn call(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(arguments)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Could not run pactl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "pactl {}: {}",
            arguments[0],
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_modules_are_selected_for_cleanup() {
        let make = |name: &str, argument: &str| Module {
            index: 1,
            name: name.into(),
            argument: argument.into(),
        };
        assert!(
            make(
                "module-null-sink",
                "sink_name=nova_control_game priority.session=0"
            )
            .owned()
        );
        assert!(!make("module-null-sink", "sink_name=nova_control_game_extra").owned());
        assert!(make("module-loopback", "source=nova_control_game.monitor sink=target sink_input_properties=application.id=nova-control.loopback").owned());
        assert!(
            !make(
                "module-loopback",
                "source=nova_control_game.monitor sink=target"
            )
            .owned()
        );
    }

    #[test]
    fn prefers_persistent_app_identity_and_excludes_our_loopback() {
        let mut stream = Stream {
            index: 12,
            sink: 2,
            corked: false,
            mute: false,
            properties: [
                ("application.name".into(), "Browser".into()),
                ("application.process.binary".into(), "firefox".into()),
            ]
            .into(),
        };
        assert_eq!(stream.app_key(), Some("firefox"));
        stream
            .properties
            .insert("application.id".into(), LOOPBACK_ID.into());
        assert_eq!(stream.app_key(), None);
    }
}
