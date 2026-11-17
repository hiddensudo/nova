use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{Channel, ChannelIcon, Pair, channel::CustomChannel};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Config {
    pub revision: u64,
    pub routes: BTreeMap<String, Channel>,
    levels: [u8; 3],
    pub pair: Option<Pair>,
    custom_channels: Vec<CustomChannel>,
    channel_order: Vec<Channel>,
    builtin_colors: [u32; 3],
    muted: BTreeSet<Channel>,
    next_channel_id: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            revision: 0,
            routes: BTreeMap::new(),
            levels: [100; 3],
            pair: None,
            custom_channels: Vec::new(),
            channel_order: Vec::new(),
            builtin_colors: [0xa994f5, 0x70c9ff, 0x79d5aa],
            muted: BTreeSet::new(),
            next_channel_id: 1,
        }
    }
}

impl Config {
    pub fn channels(&self) -> Vec<Channel> {
        let mut channels = self.channel_order.clone();
        for channel in Channel::ALL.into_iter().chain(
            self.custom_channels
                .iter()
                .map(|row| Channel::Custom(row.id)),
        ) {
            if !channels.contains(&channel) {
                channels.push(channel);
            }
        }
        channels
    }

    fn custom(&self, channel: Channel) -> Option<&CustomChannel> {
        let Channel::Custom(id) = channel else {
            return None;
        };
        self.custom_channels.iter().find(|row| row.id == id)
    }

    fn custom_mut(&mut self, channel: Channel) -> Option<&mut CustomChannel> {
        let Channel::Custom(id) = channel else {
            return None;
        };
        self.custom_channels.iter_mut().find(|row| row.id == id)
    }

    pub fn name(&self, channel: Channel) -> &str {
        self.custom(channel)
            .map_or_else(|| channel.builtin_name(), |row| row.name.as_str())
    }

    pub fn parse(&self, name: &str) -> Result<Channel, String> {
        self.channels()
            .into_iter()
            .find(|&channel| self.name(channel).eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Невідомий канал · {name}"))
    }

    pub fn level(&self, channel: Channel) -> u8 {
        channel
            .builtin_index()
            .map(|index| self.levels[index])
            .or_else(|| self.custom(channel).map(|row| row.level))
            .unwrap_or(100)
    }

    pub fn set_level(&mut self, channel: Channel, level: u8) -> Result<(), String> {
        if level > 100 {
            return Err("Гучність має бути 0–100.".into());
        }
        if let Some(index) = channel.builtin_index() {
            self.levels[index] = level;
        } else {
            self.custom_mut(channel).ok_or("Канал видалено.")?.level = level;
        }
        Ok(())
    }

    pub fn is_muted(&self, channel: Channel) -> bool {
        self.muted.contains(&channel)
    }

    #[cfg(feature = "gui")]
    pub fn set_mute(&mut self, channel: Channel, muted: bool) {
        if muted && self.exists(channel) {
            self.muted.insert(channel);
        } else {
            self.muted.remove(&channel);
        }
    }

    #[cfg(feature = "gui")]
    pub fn color(&self, channel: Channel) -> u32 {
        channel
            .builtin_index()
            .map(|index| self.builtin_colors[index])
            .or_else(|| self.custom(channel).map(|row| row.color))
            .unwrap_or(super::channel::default_color())
    }

    #[cfg(feature = "gui")]
    pub fn set_color(&mut self, channel: Channel, color: u32) -> Result<(), String> {
        if let Some(index) = channel.builtin_index() {
            self.builtin_colors[index] = color;
        } else {
            self.custom_mut(channel).ok_or("Канал видалено.")?.color = color;
        }
        Ok(())
    }

    #[cfg(feature = "gui")]
    pub fn icon(&self, channel: Channel) -> ChannelIcon {
        match channel {
            Channel::Game => ChannelIcon::Game,
            Channel::Media => ChannelIcon::Music,
            Channel::Chat => ChannelIcon::Chat,
            Channel::Custom(_) => self
                .custom(channel)
                .map_or(ChannelIcon::Globe, |row| row.icon),
        }
    }

    pub fn add_channel(
        &mut self,
        name: &str,
        color: u32,
        icon: ChannelIcon,
        index: usize,
    ) -> Result<Channel, String> {
        let name = self.validate_name(name, None)?;
        let mut order = self.channels();
        let id = self.next_channel_id;
        self.next_channel_id = id
            .checked_add(1)
            .ok_or("Досягнуто межі ідентифікаторів каналів.")?;
        let channel = Channel::Custom(id);
        self.custom_channels.push(CustomChannel {
            id,
            name,
            icon,
            color,
            level: 100,
        });
        order.insert(index.min(order.len()), channel);
        self.channel_order = order;
        Ok(channel)
    }

    #[cfg(feature = "gui")]
    pub fn edit_channel(
        &mut self,
        channel: Channel,
        name: &str,
        color: u32,
        icon: ChannelIcon,
        index: usize,
    ) -> Result<(), String> {
        self.rename_channel(channel, name)?;
        let row = self.custom_mut(channel).expect("custom channel");
        row.color = color;
        row.icon = icon;
        let mut order = self.channels();
        order.retain(|&item| item != channel);
        order.insert(index.min(order.len()), channel);
        self.channel_order = order;
        Ok(())
    }

    #[cfg(feature = "gui")]
    pub fn move_channel(&mut self, channel: Channel, before: Channel) -> Result<(), String> {
        if !self.exists(channel) || !self.exists(before) {
            return Err("Канал видалено.".into());
        }
        if channel == before {
            return Ok(());
        }
        let mut order = self.channels();
        let from = order
            .iter()
            .position(|&row| row == channel)
            .expect("existing channel");
        let to = order
            .iter()
            .position(|&row| row == before)
            .expect("existing channel");
        order.remove(from);
        order.insert(to, channel);
        self.channel_order = order;
        Ok(())
    }

    pub fn rename_channel(&mut self, channel: Channel, name: &str) -> Result<(), String> {
        let name = self.validate_name(name, Some(channel))?;
        self.custom_mut(channel)
            .ok_or("Можна перейменувати лише власний канал.")?
            .name = name;
        Ok(())
    }

    pub fn remove_channel(&mut self, channel: Channel) -> Result<(), String> {
        let Channel::Custom(id) = channel else {
            return Err("Вбудований канал не можна видалити.".into());
        };
        self.custom_channels.retain(|row| row.id != id);
        self.channel_order.retain(|&row| row != channel);
        self.routes.retain(|_, route| *route != channel);
        self.muted.remove(&channel);
        if self
            .pair
            .is_some_and(|pair| pair.left == channel || pair.right == channel)
        {
            self.pair = None;
        }
        Ok(())
    }

    fn validate_name(&self, name: &str, current: Option<Channel>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > 24
            || name.chars().any(char::is_control)
            || name.eq_ignore_ascii_case("auto")
            || self.channels().into_iter().any(|channel| {
                Some(channel) != current && self.name(channel).eq_ignore_ascii_case(name)
            })
        {
            return Err("Вкажи унікальну назву каналу від 1 до 24 символів.".into());
        }
        Ok(name.to_owned())
    }

    fn exists(&self, channel: Channel) -> bool {
        channel.builtin_index().is_some() || self.custom(channel).is_some()
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        if self.levels.iter().any(|&level| level > 100)
            || self.custom_channels.iter().any(|row| {
                row.level > 100
                    || row.id == 0
                    || row.id >= self.next_channel_id
                    || !ids.insert(row.id)
            })
            || self
                .channel_order
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != self.channel_order.len()
            || self
                .channel_order
                .iter()
                .chain(self.routes.values())
                .chain(self.muted.iter())
                .any(|&channel| !self.exists(channel))
        {
            return Err("mixer.json містить некоректний канал або рівень гучності.".into());
        }
        if let Some(pair) = self.pair {
            Pair::new(pair.left, pair.right)?;
            if !self.exists(pair.left) || !self.exists(pair.right) {
                return Err("mixer.json містить невідому пару ChatMix.".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "gui")]
    #[test]
    fn reorders_builtin_channels_without_changing_routes_or_chatmix() {
        let mut config = Config::default();
        config.routes.insert("spotify".into(), Channel::Media);
        config.pair = Some(Pair::new(Channel::Game, Channel::Media).unwrap());
        config.move_channel(Channel::Game, Channel::Chat).unwrap();
        assert_eq!(
            config.channels(),
            [Channel::Media, Channel::Chat, Channel::Game]
        );
        assert_eq!(config.routes["spotify"], Channel::Media);
        assert_eq!(config.pair.unwrap().left, Channel::Game);
        let restored: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.channels(), config.channels());
        restored.validate().unwrap();
    }

    #[test]
    fn keeps_legacy_levels_routes_and_chatmix() {
        let config: Config = serde_json::from_str(r#"{"routes":{"spotify":"media"},"levels":[84,64,76],"pair":{"left":"game","right":"media"}}"#).unwrap();
        config.validate().unwrap();
        assert_eq!(config.channels(), Channel::ALL);
        assert_eq!(config.level(Channel::Media), 64);
        assert_eq!(config.routes["spotify"], Channel::Media);
        assert_eq!(config.pair.unwrap().right, Channel::Media);
    }

    #[test]
    fn custom_identity_survives_rename_and_delete_clears_references() {
        let mut config = Config::default();
        let channel = config
            .add_channel("Browser", 0xaacfef, ChannelIcon::Globe, 1)
            .unwrap();
        config.routes.insert("browser".into(), channel);
        config.pair = Some(Pair::new(Channel::Game, channel).unwrap());
        config.set_level(channel, 48).unwrap();
        config.rename_channel(channel, "Web").unwrap();
        let mut roundtrip: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        roundtrip.validate().unwrap();
        assert_eq!(roundtrip.channels()[1], channel);
        assert_eq!(roundtrip.name(channel), "Web");
        assert_eq!(roundtrip.level(channel), 48);
        assert_eq!(roundtrip.routes["browser"], channel);
        assert_eq!(roundtrip.pair.unwrap().right, channel);
        roundtrip.remove_channel(channel).unwrap();
        assert!(!roundtrip.routes.contains_key("browser"));
        assert_eq!(roundtrip.pair, None);
        assert_ne!(
            roundtrip
                .add_channel("Web", 0, ChannelIcon::Globe, 4)
                .unwrap(),
            channel
        );
    }
}
