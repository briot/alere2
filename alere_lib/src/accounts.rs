use crate::account_kinds::AccountKindId;
use crate::institutions::InstitutionId;
use crate::multi_values::MultiValue;
use crate::transactions::{Split, TransactionRc};
use chrono::{DateTime, Local};

/// How to display account name.
/// This includes the basename for the account (level 1), its parent (level 2),
/// and so on till a maximum level.
#[derive(Clone, Copy)]
pub struct AccountNameDepth(pub usize);

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

    pub fn name(&self, acc: &Account, kind: AccountNameDepth) -> String {
        if kind.0 <= 1 {
            acc.name.clone()
        } else {
            let mut result = acc.name.clone();
            let mut remain = kind.0 - 1;
            let mut current = acc.parent;
            while remain != 0 && current.is_some() {
                let c = self.get(current.unwrap()).unwrap();
                result.insert(0, ':');
                result.insert_str(0, &c.name);
                current = c.parent;
                remain -= 1;
            }
            result
        }
    }

    pub fn postprocess(&mut self) {
        for (id, acc) in self.0.iter_mut().enumerate() {
            acc.postprocess(AccountId(id as u16 + 1));
        }
    }

    pub fn iter_accounts(&self) -> impl Iterator<Item = (AccountId, &Account)> {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, acc)| (AccountId(idx as u16 + 1), acc))
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub struct AccountId(pub u16);

impl AccountId {
    pub fn inc(&self) -> AccountId {
        AccountId(self.0 + 1)
    }
}

#[derive(Debug)]
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
pub struct Account {
    // Short name as displayed to users
    pub name: String,

    institution: Option<InstitutionId>,
    parent: Option<AccountId>,
    _description: Option<String>,

    // Only for actual IBAN, not free-form
    pub(crate) iban: Option<String>,

    // Any code used by the bank to identify the account
    _number: Option<String>,

    pub closed: bool,

    // When the account was opened
    _opened_on: Option<DateTime<Local>>,

    pub kind: AccountKindId,

    // The chronologically sorted list of transactions for which at least one
    // split applies to the account.
    transactions: Vec<TransactionRc>,

    pub(crate) reconciliations: Vec<Reconciliation>,
}

impl Account {
    #[allow(clippy::too_many_arguments)]
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
            _description: description.map(str::to_string),
            iban: iban.map(str::to_string),
            _number: number.map(str::to_string),
            closed,
            _opened_on: opened_on,
            transactions: Vec::new(),
            reconciliations: Vec::new(),
        }
    }

    pub fn set_parent(&mut self, parent: AccountId) {
        self.parent = Some(parent);
    }

    pub fn get_parent_id(&self) -> Option<AccountId> {
        self.parent
    }

    pub fn get_institution_id(&self) -> Option<InstitutionId> {
        self.institution
    }

    pub fn add_transaction(&mut self, transaction: &TransactionRc) {
        self.transactions.push(transaction.clone());
    }

    pub fn postprocess(&mut self, _self_id: AccountId) {
//        self.transactions
//            .sort_by(|tr1, tr2| tr1.earlier_than_for_account(tr2, self_id));
    }

    pub fn iter_transactions(&self) -> impl Iterator<Item = &TransactionRc> {
        self.transactions.iter()
    }

    pub fn iter_splits(
        &self,
        acc_id: AccountId,
    ) -> impl Iterator<Item = &Split> {
        self.transactions
            .iter()
            .flat_map(|tx| tx.iter_splits())
            .filter(move |s| s.account == acc_id)
    }
}
