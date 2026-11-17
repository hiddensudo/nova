use serde::{Deserialize, Serialize};

use super::Channel;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Pair {
    pub left: Channel,
    pub right: Channel,
}

impl Pair {
    pub fn new(left: Channel, right: Channel) -> Result<Self, String> {
        if left == right {
            return Err("Вибери два різні канали для ChatMix.".into());
        }
        Ok(Self { left, right })
    }

    pub(super) fn factor(self, channel: Channel, factors: [u8; 2]) -> u8 {
        if channel == self.left {
            factors[0]
        } else if channel == self.right {
            factors[1]
        } else {
            100
        }
    }
}

pub(super) fn volume(base: u8, factor: u8) -> u8 {
    ((u16::from(base) * u16::from(factor) + 50) / 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_pair_uses_base_levels_and_preserves_other_channels() {
        let custom = Channel::Custom(7);
        let pair = Pair::new(Channel::Game, custom).unwrap();
        let channels = [Channel::Game, custom, Channel::Chat];
        let apply = |values| {
            std::array::from_fn(|index| {
                volume([60, 90, 57][index], pair.factor(channels[index], values))
            })
        };
        assert_eq!(apply([100, 0]), [60, 0, 57]);
        assert_eq!(apply([100, 100]), [60, 90, 57]);
        assert_eq!(apply([0, 100]), [0, 90, 57]);
        assert_eq!(apply([50, 50]), [30, 45, 57]);
        assert!(Pair::new(Channel::Chat, Channel::Chat).is_err());
    }
}
