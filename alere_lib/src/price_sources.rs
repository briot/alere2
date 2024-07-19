#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash, Default)]
pub struct PriceSourceId(pub u16);

impl PriceSourceId {
    pub fn inc(&self) -> PriceSourceId {
        PriceSourceId(self.0 + 1)
    }
}

#[derive(Debug)]
pub struct PriceSource {
    name: String,
}

impl PriceSource {
    pub fn new(name: &str) -> Self {
        PriceSource { name: name.into() }
    }
}
