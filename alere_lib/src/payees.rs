// use std::collections::HashMap;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

/// Who money was paid to, or who paid you money
#[derive(Clone, Debug)]
pub struct Payee(Rc<RefCell<PayeeDetails>>);

impl Payee {
    pub fn get_name(&self) -> Ref<'_, String> {
        Ref::map(self.0.borrow(), |p| &p.name)
    }
}

#[derive(Default)]
pub struct PayeeCollection {
    payees: Vec<Payee>,
}

impl PayeeCollection {
    pub fn add(&mut self, name: &str) -> Payee {
        let p = Payee(Rc::new(RefCell::new(PayeeDetails {
            name: name.to_string(),
        })));
        self.payees.push(p.clone());
        p
    }
}

#[derive(Debug)]
struct PayeeDetails {
    name: String,
}
