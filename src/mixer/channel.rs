use std::borrow::Cow;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Game,
    Media,
    Chat,
    Custom(u32),
}

impl Channel {
    pub const ALL: [Self; 3] = [Self::Game, Self::Media, Self::Chat];

    pub fn builtin_index(self) -> Option<usize> {
        match self {
            Self::Game => Some(0),
            Self::Media => Some(1),
            Self::Chat => Some(2),
            Self::Custom(_) => None,
        }
    }

    pub fn builtin_name(self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Media => "Media",
            Self::Chat => "Chat",
            Self::Custom(_) => "Custom",
        }
    }

    pub(super) fn sink(self) -> Cow<'static, str> {
        match self {
            Self::Game => Cow::Borrowed("nova_control_game"),
            Self::Media => Cow::Borrowed("nova_control_media"),
            Self::Chat => Cow::Borrowed("nova_control_chat"),
            Self::Custom(id) => Cow::Owned(format!("nova_control_custom_{id}")),
        }
    }

    pub(super) fn owns_sink(name: &str) -> bool {
        if matches!(
            name,
            "nova_control_game" | "nova_control_media" | "nova_control_chat"
        ) {
            return true;
        }
        name.strip_prefix("nova_control_custom_")
            .and_then(|id| id.parse::<u32>().ok())
            .is_some_and(|id| id != 0 && Self::Custom(id).sink() == name)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelIcon {
    Game,
    Music,
    Chat,
    #[default]
    Globe,
    Stream,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct CustomChannel {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub icon: ChannelIcon,
    #[serde(default = "default_color")]
    pub color: u32,
    #[serde(default = "default_level")]
    pub level: u8,
}

pub const fn default_color() -> u32 {
    0x9cb8ff
}
const fn default_level() -> u8 {
    100
}
