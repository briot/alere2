#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub struct PayeeId(pub u32);

impl PayeeId {
    pub fn inc(&self) -> PayeeId {
        PayeeId(self.0 + 1)
    }
}

/// Who money was paid to, or who paid you money

#[derive(Debug)]
pub struct Payee {
    pub name: String,
}

impl Payee {
    pub fn new(name: &str) -> Self {
        Payee { name: name.into() }
    }
}
