use crate::accounts::AccountId;
use crate::multi_values::{MultiValue, Operation};
use crate::payees::PayeeId;
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
    splits: Vec<Split>,
}

impl Transaction {
    /// Check that the transaction obeys the accounting equations, i.e.
    ///    Equity = Assets + Income − Expenses
    pub fn is_balanced(&self) -> bool {
        true
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
        account: AccountId,
        reconciled: ReconcileKind,
        post_ts: DateTime<Local>,
        original_value: Operation,
        value: Option<MultiValue>,
    ) -> &mut Split {
        let split = Split {
            account,
            reconciled,
            post_ts,
            original_value,
            value,
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

    fn timestamp_for_account(
        &self,
        account: AccountId,
    ) -> Option<DateTime<Local>> {
        for s in &self.0.splits {
            if s.account == account {
                return Some(s.post_ts);
            }
        }
        None
    }

    pub fn earlier_than_for_account(
        &self,
        right: &TransactionRc,
        account: AccountId,
    ) -> std::cmp::Ordering {
        let t1 = self.timestamp_for_account(account);
        let t2 = right.timestamp_for_account(account);
        t1.cmp(&t2)
    }
}

/// GnuCash and Kmymoney call these splits.
/// Beancount and Ledger call these postings.
#[derive(Debug)]
pub struct Split {
    // Which account is impacted bu this split.
    pub account: AccountId,

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

    // The amount of the split, in the original currency
    // This is potentially given in another currency or commodity.
    pub original_value: Operation,

    // The amount of the transaction as made originally.
    // The goal is to support multi-currency transactions.
    // Here are various ways this value can be used:
    //
    // * a 1000 EUR transaction for an account in EUR. In this case, value is
    //   useless and does not provide any additional information.
    //       original_value = 1000 EUR  (scaled)
    //       value          = 1000 EUR  (scaled)
    //
    // * an ATM operation of 100 USD for the same account in EUR while abroad.
    //   Exchange rate at the time: 0.85 EUR = 1 USD.  Also assume there is a
    //   bank fee that applies.
    //      split1: account=checking account
    //              original_value = -100 USD    (actually withdrawn)
    //              value          = -85 EUR   (as shown on your bank statement)
    //      split2: account=expense:cash  value= +84.7 EUR  original= +84.7 EUR
    //      split3: account=expense:fees  value= +0.3 EUR   original= +0.3 EUR
    //   So value is used to show you exactly the amount you manipulated. The
    //   exchange rate can be computed from qty and value.
    //
    // * Buying 10 shares for AAPL at 120 USD. There are several splits here,
    //   one where we increase the number of shares in the STOCK account.
    //   The money came from an investment account in EUR, which has its own
    //   split for the same transaction:
    //       split1: account=stock       value=1200 USD   original=10 AAPL
    //       split2: account=investment  value=-1200 USD  original=-1020 EUR

    // ??? Should we use an Operation as well, though it would be in a different
    // currency
    pub value: Option<MultiValue>,
}

#[cfg(test)]
mod test {
    use crate::accounts::AccountId;
    use crate::commodities::CommodityId;
    use crate::errors::AlrError;
    use crate::multi_values::MultiValue;
    use crate::transactions::{Operation, ReconcileKind, TransactionRc};
    use chrono::Local;
    use rust_decimal_macros::dec;

    #[test]
    fn test_proper() -> Result<(), AlrError> {
        let mut tr = TransactionRc::new_with_default();
        tr.add_split(
            AccountId(1),
            ReconcileKind::New,
            Local::now(),
            Operation::Credit(MultiValue::new(dec!(1.1), CommodityId(1))),
            None,
        );

        assert!(tr.is_balanced());
        Ok(())
    }
}
