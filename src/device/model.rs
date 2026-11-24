#[derive(Clone, Copy)]
pub(super) enum StatusFormat {
    Gen2,
    Gen1Percent,
    Gen1Steps,
}

#[derive(Clone, Copy)]
pub(super) struct Model {
    product: u16,
    name: &'static str,
    pub status_format: StatusFormat,
}

impl Model {
    pub const VENDOR: u16 = 0x1038;
    pub const NOVA_7_GEN2: Self = Self::new(0x227e, "Arctis Nova 7 Gen 2", StatusFormat::Gen2);
    pub const PROBE_MODELS: [Self; 7] = [
        Self::NOVA_7_GEN2,
        Self::new(0x229e, "Arctis Nova 7X Gen 2", StatusFormat::Gen2),
        Self::new(0x2202, "Arctis Nova 7 Gen 1", StatusFormat::Gen1Steps),
        Self::new(0x22a1, "Arctis Nova 7 Gen 1", StatusFormat::Gen1Percent),
        Self::new(0x2206, "Arctis Nova 7X Gen 1", StatusFormat::Gen1Steps),
        Self::new(0x22a4, "Arctis Nova 7X Gen 1", StatusFormat::Gen1Steps),
        Self::new(0x22a5, "Arctis Nova 7X Gen 1", StatusFormat::Gen1Percent),
    ];

    const fn new(product: u16, name: &'static str, status_format: StatusFormat) -> Self {
        Self {
            product,
            name,
            status_format,
        }
    }

    pub fn product(self) -> u16 {
        self.product
    }

    pub fn name(self) -> &'static str {
        self.name
    }

    pub fn for_product(vendor: u16, product: u16) -> Option<Self> {
        Self::PROBE_MODELS
            .into_iter()
            .find(|model| model.matches(vendor, product))
    }

    pub fn matches(self, vendor: u16, product: u16) -> bool {
        vendor == Self::VENDOR && product == self.product
    }
}
