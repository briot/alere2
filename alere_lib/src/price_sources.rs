use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct PriceSourceId(u8);

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum PriceSourceFrom {
    // The price was computed from a transaction
    Transaction,

    // Computing using a turnkey
    Turnkey,

    // The price was downloaded from an external price source
    External(PriceSourceId),
}

#[derive(Clone, Debug)]
pub struct PriceSource(Rc<RefCell<PriceSourceDetails>>);

impl PriceSource {
    pub fn get_name(&self) -> Ref<'_, String> {
        Ref::map(self.0.borrow(), |p| &p.name)
    }

    pub fn get_id(&self) -> PriceSourceId {
        self.0.borrow().id
    }
}

#[derive(Default)]
pub struct PriceSourceCollection {
    sources: HashMap<PriceSourceId, PriceSource>,
}

impl PriceSourceCollection {
    pub fn add(&mut self, name: &str) -> PriceSource {
        let id = PriceSourceId(
            self.sources
                .values()
                .map(|s| s.0.borrow().id.0)
                .max()
                .unwrap_or(0)
                + 1,
        );
        let s = PriceSource(Rc::new(RefCell::new(PriceSourceDetails {
            id,
            name: name.to_string(),
        })));
        self.sources.insert(id, s.clone());
        s
    }
}

#[derive(Debug)]
struct PriceSourceDetails {
    id: PriceSourceId, // unique persistent id
    name: String,
}
