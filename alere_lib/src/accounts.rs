use crate::account_kinds::AccountKindId;
use crate::institutions::InstitutionId;
use chrono::{DateTime, Local};

pub enum AccountNameKind {
    Short,
    Full,
}

#[derive(Default)]
pub struct AccountCollection(Vec<Account>);

impl AccountCollection {
    pub fn add(&mut self, account: Account) -> AccountId {
        self.0.push(account);
        AccountId(self.0.len() as u16)
    }

    pub fn get_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.0.get_mut(id.0 as usize - 1)
    }

    pub fn get(&self, id: AccountId) -> Option<&Account> {
        self.0.get(id.0 as usize - 1)
    }

    pub fn name(&self, id: AccountId, kind: AccountNameKind) -> String {
        match kind {
            AccountNameKind::Short => {
                let acc = self.get(id).unwrap();
                acc.name.clone()
            }
            AccountNameKind::Full => {
                let acc = self.get(id).unwrap();
                if let Some(p) = acc.parent {
                    format!("{}::{}", self.name(p, kind), acc.name)
                } else {
                    acc.name.clone()
                }
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub struct AccountId(pub u16);

impl AccountId {
    pub fn inc(&self) -> AccountId {
        AccountId(self.0 + 1)
    }
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

#[derive(Debug)]
pub struct Account {
    // Short name as displayed to users
    name: String,

    institution: Option<InstitutionId>,
    parent: Option<AccountId>,
    description: Option<String>,

    // Only for actual IBAN, not free-form
    iban: Option<String>,

    // Any code used by the bank to identify the account
    number: Option<String>,

    pub closed: bool,

    // When the account was opened
    opened_on: Option<DateTime<Local>>,

    pub kind: AccountKindId,
}

impl Account {
    pub fn new(
        name: &str,
        kind: AccountKindId,
        parent: Option<AccountId>,
        institution: Option<InstitutionId>,
        description: Option<&str>,
        iban: Option<&str>,
        number: Option<&str>,
        closed: bool,
        opened_on: Option<DateTime<Local>>,
    ) -> Self {
        Account {
            name: name.into(),
            kind,
            parent,
            institution,
            description: description.map(str::to_string),
            iban: iban.map(str::to_string),
            number: number.map(str::to_string),
            closed,
            opened_on,
        }
    }

    pub fn set_parent(&mut self, parent: AccountId) {
        self.parent = Some(parent);
    }
}
