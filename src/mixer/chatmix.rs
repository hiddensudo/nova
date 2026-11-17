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

    pub(super) fn levels(self, mut base: [u8; 3], factors: [u8; 2]) -> [u8; 3] {
        for (channel, factor) in [self.left, self.right].into_iter().zip(factors) {
            let level = &mut base[channel as usize];
            *level = ((u16::from(*level) * u16::from(factor) + 50) / 100) as u8;
        }
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_pair_uses_base_levels_and_preserves_third_channel() {
        let pair = Pair::new(Channel::Game, Channel::Media).unwrap();
        assert_eq!(pair.levels([60, 90, 57], [100, 0]), [60, 0, 57]);
        assert_eq!(pair.levels([60, 90, 57], [100, 100]), [60, 90, 57]);
        assert_eq!(pair.levels([60, 90, 57], [0, 100]), [0, 90, 57]);
        assert_eq!(pair.levels([60, 90, 57], [50, 50]), [30, 45, 57]);
        assert!(Pair::new(Channel::Chat, Channel::Chat).is_err());
    }
}
