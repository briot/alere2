#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum PriceSourceId {
    // The price was computed from a transaction
    Transaction,

    // The price was downloaded from an external price source
    External(u16),
}

impl PriceSourceId {
    pub fn inc(&self) -> PriceSourceId {
        match self {
            PriceSourceId::Transaction => {
                panic!("Cannot increase PriceSource::Transaction")
            }
            PriceSourceId::External(id) => PriceSourceId::External(id + 1),
        }
    }
}

#[derive(Debug)]
pub struct PriceSource {
    _name: String,
}

impl PriceSource {
    pub fn new(name: &str) -> Self {
        PriceSource { _name: name.into() }
    }
}
