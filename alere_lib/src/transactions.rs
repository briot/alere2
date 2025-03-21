use crate::{
    accounts::{Account, AccountNameDepth},
    errors::AlrError,
    multi_values::{MultiValue, Operation, Value},
    payees::Payee,
};
use anyhow::Result;
use chrono::{DateTime, Local};
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

#[derive(Debug)]
pub enum ReconcileKind {
    // A newly added transaction
    New,

    // When the split has been seen on a bank statement
    Cleared,

    // When the split has been checked by the user, and the overall result of
    // a bank statement matches all previously reconciled + currently cleared
    // transactions.
    Reconciled(Option<DateTime<Local>>),
}

/// Details used to build a new transaction.
/// Such objects are short-lived.  All fields are made public and with default,
/// so one can use
///     TransactionRc::new(TransactionDetails {
///         memo: Some("ssdsd"),
///         ..Default::default()
///     })
///
/// The transaction has no timestamp of its own.  Instead, it should use the
/// earliest timestamp among all its splits.
/// TODO The transaction could be entered before any of its splits takes effect,
/// but since it then has no impact on balance it doesn't seem worth remembering
/// that date.

#[derive(Default)]
pub struct TransactionArgs<'a> {
    pub memo: Option<&'a str>,
    pub check_number: Option<&'a str>,

    // The payee.  Though a transaction is made of multiple splits, users
    // typically think of it as applying to a single payee, even though extra
    // splits might involve taxes for instance.
    pub payee: Option<Payee>,

    // When was the transaction entered in the database.  This might be totally
    // different from the split's timestamp (which are when the split impacted
    // the corresponding account).
    pub entry_date: DateTime<Local>,
    // ??? how does this apply to splits, which contain the timestamp
    //    pub scheduled: Option<String>,
    //    pub last_occurrence: Option<DateTime<Local>>,
    //    pub scenario_id: Scenario,
}

#[derive(Debug, Default)]
struct TransactionDetails {
    memo: Option<String>,
    check_number: Option<String>,
    payee: Option<Payee>,
    _entry_date: DateTime<Local>,

    // The splits that make up the transaction.  The sum of these splits must
    // always be balanced.  The transaction owns the splits.
    // ??? Those splits are sorted for the sake of Operation::Split.  If the
    // latter had "the new number of shares" we would not need the sorting.
    splits: Vec<Split>,
}

#[derive(Debug, Clone)]
pub struct Transaction(Rc<RefCell<TransactionDetails>>);

impl Transaction {
    // Create a new transaction with default fields

    pub fn new_with_details(details: TransactionArgs) -> Self {
        Transaction(Rc::new(RefCell::new(TransactionDetails {
            memo: details.memo.and_then(|m| {
                if m.is_empty() {
                    None
                } else {
                    Some(m.into())
                }
            }),
            check_number: details.check_number.map(str::to_string),
            payee: details.payee,
            _entry_date: details.entry_date,
            splits: Vec::default(),
        })))
    }

    pub fn new_with_default() -> Self {
        Transaction(Rc::new(RefCell::new(TransactionDetails::default())))
    }

    /// Create a new split for this transaction
    pub fn add_split(
        &mut self,
        account: Account,
        reconciled: ReconcileKind,
        post_ts: DateTime<Local>,
        operation: Operation,
    ) {
        let split = Split {
            account,
            reconciled,
            post_ts,
            operation,
        };
        let mut tr = Rc::get_mut(&mut self.0)
            .expect("Couldn'get get mut ref to transation")
            .borrow_mut();
        tr.splits.push(split);
    }

    /// Check that the transaction obeys the accounting equations, i.e.
    ///    Equity = Assets + Income − Expenses
    pub fn is_balanced(&self) -> bool {
        let mut total = MultiValue::zero();
        for s in &self.0.borrow().splits {
            match &s.operation {
                Operation::Credit(value) => {
                    total += value;
                }
                Operation::AddShares { qty } => {
                    total += qty;
                }
                Operation::BuyAmount { amount, .. } => {
                    total += amount;
                }
                Operation::BuyPrice { qty, price } => {
                    total += &Value {
                        amount: qty.amount * price.amount,
                        commodity: price.commodity.clone(),
                    };
                }
                Operation::Reinvest { amount, .. } => {
                    total += amount;
                }
                Operation::Split { .. } => {}
                Operation::Dividend => {}
            }
            // total.apply(&s.operation);
        }
        total.is_zero()
    }

    pub fn set_check_number(
        &mut self,
        check_number: Option<&str>,
    ) -> Result<(), String> {
        match check_number {
            None | Some("") => {}
            Some(num) => {
                let is_none = self.0.borrow().check_number.is_none();
                if is_none {
                    self.0.borrow_mut().check_number = Some(num.into());
                } else if let Some(old) = &self.0.borrow().check_number {
                    if old != num {
                        Err(format!(
                            "Non-matching check number, had {old:?}, now {num}"
                        ))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_memo(&mut self, memo: Option<&str>) {
        match memo {
            None | Some("") => {}
            Some(memo) => {
                let is_same = match &self.0.borrow().memo {
                    None => false,
                    Some(old) => old == memo,
                };

                // ??? kmymoney has memo for the transaction (which
                // seems to be the initial payee as downloaded), then
                // one memory per split.  The transaction's memo doesn't
                // to be editable, so we keep the memo from the split
                // instead.
                if !is_same {
                    self.0.borrow_mut().memo = Some(memo.into());
                }
            }
        }
    }

    pub fn set_payee(&mut self, payee: Option<&Payee>) {
        match payee {
            None => {}
            Some(payee) => {
                // ??? kmymoney allows different payees for each
                // split.  We only keep the first one.
                let should_set = self.0.borrow().payee.is_none();
                if should_set {
                    self.0.borrow_mut().payee = Some(payee.clone());
                }
            }
        }
    }

    pub fn splits(&self) -> Ref<'_, Vec<Split>> {
        Ref::map(self.0.borrow(), |tx| &tx.splits)
    }

    /// Find a memo or description for the transaction, possibly looking into
    /// splits themselves.
    pub fn memo(&self) -> Ref<'_, Option<String>> {
        Ref::map(self.0.borrow(), |tx| &tx.memo)
    }

    pub fn timestamp_for_account(
        &self,
        account: &Account,
    ) -> Option<DateTime<Local>> {
        for s in &self.0.borrow().splits {
            if s.account == *account {
                return Some(s.post_ts);
            }
        }
        None
    }

    /// Return the timestamp to use for the transaction.  This is computed
    /// as the oldest timestamp among all the splits
    pub fn timestamp(&self) -> DateTime<Local> {
        self.splits()
            .iter()
            .map(|s| s.post_ts)
            .min()
            .unwrap_or(Local::now())
    }
}

#[derive(Default)]
pub struct TransactionCollection {
    /// List of transactions, kept sorted
    tx: Vec<Transaction>,
}

impl TransactionCollection {
    /// Registers a transaction, which must be sorted.
    /// It is also added to all relevant accounts.
    pub fn add(&mut self, tr: Transaction) -> Result<()> {
        if !tr.is_balanced() {
            Err(AlrError::Str(format!("Transaction not balanced: {:?}", tr)))?;
        }

        for s in tr.splits().iter() {
            // Add the transaction to each account it applies to
            s.account.add_transaction(&tr);
        }

        let tx_stamp = tr.timestamp();

        // Add sorted
        let pos =
            match self.tx.binary_search_by(|t| t.timestamp().cmp(&tx_stamp)) {
                Ok(pos) | Err(pos) => pos,
            };
        self.tx.insert(pos, tr);
        Ok(())
    }

    /// Return all transactions, sorted by timestamp
    pub fn iter(&self) -> impl Iterator<Item = &Transaction> {
        self.tx.iter()
    }
}

/// GnuCash and Kmymoney call these splits.
/// Beancount and Ledger call these postings.
pub struct Split {
    // Which account is impacted bu this split.
    pub account: Account,

    // When was this split reconciled
    pub reconciled: ReconcileKind,

    // Timestamp is associated with each split.  Although the user will most
    // often only specify the date, we still include full timestamp here for
    // the sake of ordering, or possibly supporting intra-day trading at some
    // point.
    //
    // For instance, if we have a transfer between two banks, the transaction
    // would include two splits:
    //   * Transaction 1
    //      - Split1  2024-01-01  account1   -100 EUR
    //      - Split2  2024-01-04  account2   +100 EUR
    // However, this means that this transaction is not balanced (total equal
    // to zero) between these two dates.  So internally, this transaction
    // behaves as:
    //   * Transaction 1 internal
    //      - Split1  2024-01-01  account1          -100 EUR
    //      - Split3  2024-01-01  equity::transfer  +100 EUR
    //      - Split2  2024-01-04  account2          +100 EUR
    //      - Split4  2024-01-04  equity::transfer  -100 EUR
    // kmymoney and gnucash seem to have the same flexibility in their database,
    // but not take advantage of it in the user interface.
    pub post_ts: DateTime<Local>,

    pub operation: Operation,
}

impl core::fmt::Debug for Split {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Split")
            .field("operation", &self.operation)
            .field("account", &self.account.name(AccountNameDepth::basename()))
            .field("reconciled", &self.reconciled)
            .field("post_ts", &self.post_ts)
            .finish()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        account_categories::AccountCategory,
        account_kinds::AccountKind,
        accounts::AccountCollection,
        commodities::CommodityCollection,
        errors::AlrError,
        multi_values::{MultiValue, Operation},
        transactions::{ReconcileKind, Transaction},
    };
    use chrono::Local;
    use rust_decimal_macros::dec;

    #[test]
    fn test_proper() -> Result<(), AlrError> {
        let mut tr = Transaction::new_with_default();
        let mut coms = CommodityCollection::default();
        let mut accounts = AccountCollection::default();
        let comm = coms.add_dummy("euro", false);
        let kind =
            AccountKind::new("eee", "Inc", "Dec", AccountCategory::EXPENSE);
        tr.add_split(
            accounts.add_dummy("aaa", kind.clone()),
            ReconcileKind::New,
            Local::now(),
            Operation::Credit(MultiValue::new(dec!(1.1), &comm)),
        );
        assert!(!tr.is_balanced());

        tr.add_split(
            accounts.add_dummy("bbb", kind.clone()),
            ReconcileKind::New,
            Local::now(),
            Operation::Credit(MultiValue::new(dec!(-1.1), &comm)),
        );
        assert!(tr.is_balanced());

        Ok(())
    }
}
