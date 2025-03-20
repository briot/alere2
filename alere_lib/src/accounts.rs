use crate::{
    account_kinds::AccountKind,
    institutions::Institution,
    multi_values::MultiValue,
    transactions::{Split, TransactionRc},
};
use chrono::{DateTime, Local};
use std::{cell::RefCell, rc::Rc};

/// How to display account name.
/// This includes the basename for the account (level 1), its parent (level 2),
/// and so on till a maximum level.
#[derive(Clone, Copy)]
pub struct AccountNameDepth(usize);
impl AccountNameDepth {
    /// Only show the basename of the account
    pub fn basename() -> Self {
        AccountNameDepth(1)
    }

    /// Show all levels
    pub fn unlimited() -> Self {
        AccountNameDepth(usize::MAX)
    }

    /// Only show a limited number of levels
    pub fn limit(max: usize) -> Self {
        AccountNameDepth(max)
    }

    /// Increase the limit
    pub fn inc(&self, increment: usize) -> Self {
        if self.0 == usize::MAX {
            AccountNameDepth::unlimited()
        } else {
            AccountNameDepth(self.0 + increment)
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default, PartialOrd, Ord)]
pub struct AccountId(pub u16);

impl AccountId {
    pub fn inc(&self) -> AccountId {
        AccountId(self.0 + 1)
    }
}

#[derive(Clone, Debug)]
pub struct Reconciliation {
    pub timestamp: DateTime<Local>,
    pub total: MultiValue,
}

/// Either an actual bank account, or a category.
/// All accounts must be children of one of the five root accounts:
///    - Assets:    what the user owns
///    - Liability: what the user owes
///    - Equity:    what the world owes you (opening balances, transfers,...)
///    - Revenue
///    - Expenses
///
/// An account can contain multiple commodities.  For instance, an account
/// representing your employer could be used both for your salaries (what your
/// employer paid in exchange of your work) and for accrued vacations.
/// Or a brokerage account could contain both cash (USD) and shares of one or
/// more stocks.   An alternative is to create multiple children accounts, one
/// per commodity, but the flexibility is left to the user.

#[derive(Debug)] // NOT clone-able
struct AccountDetails {
    // Unique id for the lifetime of the repository.  We never reassign ids.
    id: AccountId,

    // Short name as displayed to users
    name: String,

    institution: Option<Institution>,
    parent: Option<Account>,
    _description: Option<String>,

    // Only for actual IBAN, not free-form
    iban: Option<String>,

    // Any code used by the bank to identify the account
    _number: Option<String>,

    closed: bool,

    // When the account was opened
    _opened_on: Option<DateTime<Local>>,

    kind: AccountKind,

    // The chronologically sorted list of transactions for which at least one
    // split applies to the account.
    transactions: Vec<TransactionRc>,

    reconciliations: Vec<Reconciliation>,
}

#[derive(Clone, Debug)]
pub struct Account(Rc<RefCell<AccountDetails>>);

impl Account {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: &str,
        kind: AccountKind,
        parent: Option<Account>,
        institution: Option<Institution>,
        description: Option<&str>,
        iban: Option<&str>,
        number: Option<&str>,
        closed: bool,
        opened_on: Option<DateTime<Local>>,
    ) -> Self {
        Account(Rc::new(RefCell::new(AccountDetails {
            id: AccountId::default(), // set when registered in repository
            name: name.into(),
            kind,
            parent,
            institution,
            _description: description.map(str::to_string),
            iban: iban.map(str::to_string),
            _number: number.map(str::to_string),
            closed,
            _opened_on: opened_on,
            transactions: Vec::new(),
            reconciliations: Vec::new(),
        })))
    }

    #[cfg(test)]
    pub fn new_dummy(name: &str, kind: AccountKind) -> Self {
        Account::new(name, kind, None, None, None, None, None, false, None)
    }

    fn name_internal(&self, kind: AccountNameDepth, into: &mut String) {
        if kind.0 > 1 {
            if let Some(p) = &self.0.borrow().parent {
                p.name_internal(AccountNameDepth(kind.0 - 1), into);
                into.push(':');
            }
        }
        into.push_str(&self.0.borrow().name);
    }

    /// Return the name of the account, including parents
    pub fn name(&self, kind: AccountNameDepth) -> String {
        let mut result = String::new();
        self.name_internal(kind, &mut result);
        result
    }

    pub fn set_parent(&mut self, parent: Account) {
        self.0.borrow_mut().parent = Some(parent);
    }

    pub fn get_parent(&self) -> Option<Account> {
        self.0.borrow().parent.clone()
    }

    /// Return the institution to which an account belongs.  If the account
    /// itself doesn't specify this information, look in the parent account.
    pub fn get_institution(&self) -> Option<Institution> {
        match &self.0.borrow().institution {
            None => match &self.0.borrow().parent {
                None => None,
                Some(p) => p.get_institution(),
            },
            Some(inst) => Some(inst.clone()),
        }
    }

    /// Register a transaction for which one of the splits applies to self.
    /// It keeps the list of transactions sorted.
    pub fn add_transaction(&self, transaction: &TransactionRc) {
        //  ??? Fails because when we look at all the splits, we try to borrow
        //  the AccountRc that contains self, and self is already borrowed as
        //  mutable.
        match transaction.timestamp_for_account(self) {
            None => {
                panic!(
                    "Could not insert irrelevant transaction {:?} \
                        in account {:?}",
                    transaction, self
                );
            }
            Some(d) => {
                let pos =
                    match self.0.borrow().transactions.binary_search_by(|tx| {
                        tx.timestamp_for_account(self).cmp(&Some(d))
                    }) {
                        Ok(pos) | Err(pos) => pos,
                    };
                self.0
                    .borrow_mut()
                    .transactions
                    .insert(pos, transaction.clone());
            }
        }
    }

    pub fn iter_transactions(
        &self,
    ) -> impl Iterator<Item = TransactionRc> + '_ {
        struct Iter<'b> {
            account: &'b Account,
            index: usize,
        }
        impl Iterator for Iter<'_> {
            type Item = TransactionRc;

            fn next(&mut self) -> Option<Self::Item> {
                let pos = self.index;
                self.index += 1;
                self.account.0.borrow().transactions.get(pos).cloned()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = self.account.0.borrow().transactions.len();
                let size = len.saturating_sub(self.index);
                (size, Some(size))
            }
        }

        Iter {
            account: self,
            index: 0,
        }
    }

    pub fn for_each_split<F>(&self, mut cb: F)
    where
        F: FnMut(&Split),
    {
        self.iter_transactions().for_each(|tx| {
            tx.iter_splits_for_account(self.clone()).for_each(&mut cb)
        });
    }

    pub fn cmp_name(&self, right: &Account) -> std::cmp::Ordering {
        self.0.borrow().name.cmp(&right.0.borrow().name)
    }

    pub fn get_kind(&self) -> AccountKind {
        self.0.borrow().kind.clone()
    }

    pub fn set_id(&mut self, id: AccountId) {
        self.0.borrow_mut().id = id;
    }

    pub fn get_id(&self) -> AccountId {
        self.0.borrow().id
    }

    pub fn set_iban(&mut self, iban: &str) {
        self.0.borrow_mut().iban = Some(iban.to_string());
    }

    pub fn add_reconciliation(&mut self, rec: Reconciliation) {
        self.0.borrow_mut().reconciliations.push(rec);
    }

    pub fn for_each_reconciliation<F>(&self, mut cb: F)
    where
        F: FnMut(&Reconciliation),
    {
        for r in &self.0.borrow().reconciliations {
            cb(r);
        }
    }

    pub fn iter_reconciliations(
        &self,
    ) -> impl Iterator<Item = Reconciliation> + '_ {
        struct Iter<'b> {
            account: &'b Account,
            index: usize,
        }
        impl Iterator for Iter<'_> {
            type Item = Reconciliation;

            fn next(&mut self) -> Option<Self::Item> {
                let pos = self.index;
                self.index += 1;
                self.account.0.borrow().reconciliations.get(pos).cloned()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = self.account.0.borrow().reconciliations.len();
                let size = len.saturating_sub(self.index);
                (size, Some(size))
            }
        }

        Iter {
            account: self,
            index: 0,
        }
    }

    pub fn close(&mut self) {
        self.0.borrow_mut().closed = true;
    }
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }
}

impl Eq for Account {}
