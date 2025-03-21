use crate::{
    accounts::Account,
    multi_values::{MultiValue, Operation, Value},
    payees::PayeeId,
};
use chrono::{DateTime, Local};
use std::rc::Rc;

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
pub struct TransactionDetails<'a> {
    pub memo: Option<&'a str>,
    pub check_number: Option<&'a str>,

    // The payee.  Though a transaction is made of multiple splits, users
    // typically think of it as applying to a single payee, even though extra
    // splits might involve taxes for instance.
    pub payee: Option<PayeeId>,

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
pub struct Transaction {
    memo: Option<String>,
    check_number: Option<String>,
    payee: Option<PayeeId>,
    _entry_date: DateTime<Local>,

    // The splits that make up the transaction.  The sum of these splits must
    // always be balanced.  The transaction owns the splits.
    // ??? Those splits are sorted for the sake of Operation::Split.  If the
    // latter had "the new number of shares" we would not need the sorting.
    splits: Vec<Split>,
}

impl Transaction {
    /// Check that the transaction obeys the accounting equations, i.e.
    ///    Equity = Assets + Income − Expenses
    pub fn is_balanced(&self) -> bool {
        let mut total = MultiValue::zero();
        for s in &self.splits {
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
}

#[derive(Debug, Clone)]
pub struct TransactionRc(Rc<Transaction>);

impl TransactionRc {
    // Create a new transaction with default fields

    pub fn new_with_details(details: TransactionDetails) -> Self {
        TransactionRc(Rc::new(Transaction {
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
        }))
    }

    pub fn new_with_default() -> Self {
        TransactionRc(Rc::new(Transaction {
            ..Default::default()
        }))
    }

    /// Create a new split for this transaction
    pub fn add_split(
        &mut self,
        account: Account,
        reconciled: ReconcileKind,
        post_ts: DateTime<Local>,
        operation: Operation,
    ) -> &mut Split {
        let split = Split {
            account,
            reconciled,
            post_ts,
            operation,
        };
        let tr = Rc::get_mut(&mut self.0)
            .expect("Couldn'get get mut ref to transation");
        tr.splits.push(split);
        tr.splits.last_mut().unwrap()
    }

    pub fn is_balanced(&self) -> bool {
        self.0.is_balanced()
    }

    pub fn set_check_number(
        &mut self,
        check_number: Option<&str>,
    ) -> Result<(), String> {
        match check_number {
            None | Some("") => {}
            Some(num) => match &self.0.check_number {
                None => {
                    Rc::get_mut(&mut self.0).unwrap().check_number =
                        Some(num.into())
                }
                Some(old) if old == num => {}
                Some(old) => {
                    Err(format!(
                        "Non-matching check number, had {old:?}, now {num}"
                    ))?;
                }
            },
        }
        Ok(())
    }

    pub fn set_memo(&mut self, memo: Option<&str>) {
        match memo {
            None | Some("") => {}
            Some(memo) => match &self.0.memo {
                None => {
                    Rc::get_mut(&mut self.0).unwrap().memo = Some(memo.into())
                }
                Some(old) if old == memo => {}
                Some(_) => {
                    // ??? kmymoney has memo for the transaction (which
                    // seems to be the initial payee as downloaded), then
                    // one memory per split.  The transaction's memo doesn't
                    // to be editable, so we keep the memo from the split
                    // instead.
                    Rc::get_mut(&mut self.0).unwrap().memo = Some(memo.into());
                }
            },
        }
    }

    pub fn set_payee(&mut self, payee: Option<&PayeeId>) {
        match payee {
            None => {}
            Some(payee) => {
                match &self.0.payee {
                    None => {
                        Rc::get_mut(&mut self.0).unwrap().payee = Some(*payee)
                    }
                    Some(old) if old == payee => {}
                    Some(_) => {
                        // ??? kmymoney allows different payees for each
                        // split.  We only keep the first one.
                        // println!(
                        //     "{tid}/{sid}: Non-matching payee, had {old:?}, now {p:?}"
                        // );
                    }
                }
            }
        }
    }

    pub fn iter_splits(&self) -> impl std::iter::Iterator<Item = &Split> {
        self.0.splits.iter()
    }

    pub fn iter_splits_for_account(
        &self,
        account: Account,
    ) -> impl std::iter::Iterator<Item = &Split> {
        self.0.splits.iter().filter(move |s| s.account == account)
    }

    /// Find a memo or description for the transaction, possibly looking into
    /// splits themselves.
    pub fn memo(&self) -> Option<&str> {
        if self.0.memo.is_some() {
            return self.0.memo.as_deref();
        }
        None
    }

    pub fn timestamp_for_account(
        &self,
        account: &Account,
    ) -> Option<DateTime<Local>> {
        for s in &self.0.splits {
            if s.account == *account {
                return Some(s.post_ts);
            }
        }
        None
    }
}

/// GnuCash and Kmymoney call these splits.
/// Beancount and Ledger call these postings.
#[derive(Debug)]
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

#[cfg(test)]
mod test {
    use crate::{
        account_categories::AccountCategory,
        account_kinds::AccountKind,
        accounts::Account,
        commodities::CommodityCollection,
        errors::AlrError,
        multi_values::{MultiValue, Operation},
        transactions::{ReconcileKind, TransactionRc},
    };
    use chrono::Local;
    use rust_decimal_macros::dec;

    #[test]
    fn test_proper() -> Result<(), AlrError> {
        let mut tr = TransactionRc::new_with_default();
        let mut coms = CommodityCollection::default();
        let comm = coms.add_dummy("euro", false);
        let kind =
            AccountKind::new("eee", "Inc", "Dec", AccountCategory::EXPENSE);
        tr.add_split(
            Account::new_dummy("aaa", kind.clone()),
            ReconcileKind::New,
            Local::now(),
            Operation::Credit(MultiValue::new(dec!(1.1), &comm)),
        );
        assert!(!tr.is_balanced());

        tr.add_split(
            Account::new_dummy("bbb", kind.clone()),
            ReconcileKind::New,
            Local::now(),
            Operation::Credit(MultiValue::new(dec!(-1.1), &comm)),
        );
        assert!(tr.is_balanced());

        Ok(())
    }
}
