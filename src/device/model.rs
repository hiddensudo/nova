#[derive(Clone, Copy)]
pub(super) enum Model {
    Nova7Gen2,
    Nova7xGen2,
}

impl Model {
    pub const VENDOR: u16 = 0x1038;

    pub fn product(self) -> u16 {
        match self {
            Self::Nova7Gen2 => 0x227e,
            Self::Nova7xGen2 => 0x229e,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Nova7Gen2 => "Arctis Nova 7 Gen 2",
            Self::Nova7xGen2 => "Arctis Nova 7X Gen 2",
        }
    }

    pub fn matches(self, vendor: u16, product: u16) -> bool {
        vendor == Self::VENDOR && product == self.product()
    }
}
