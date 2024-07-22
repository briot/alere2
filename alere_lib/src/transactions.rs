use crate::accounts::AccountId;
use crate::commodities::CommodityId;
use crate::multi_values::Value;
use crate::payees::PayeeId;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;

//  type Scenario = u32;

#[derive(Debug)]
pub enum ReconcileKind {
    // A newly added transaction
    NEW,

    // When the split has been seen on a bank statement
    CLEARED,

    // When the split has been checked by the user, and the overall result of
    // a bank statement matches all previously reconciled + currently cleared
    // transactions.
    RECONCILED(Option<DateTime<Local>>),
}

/// The transaction has no timestamp of its own.  Instead, it should use the
/// earliest timestamp among all its splits.
/// TODO The transaction could be entered before any of its splits takes effect,
/// but since it then has no impact on balance it doesn't seem worth remembering
/// that date.

#[derive(Debug)]
pub struct Transaction {
    pub memo: Option<String>,
    pub check_number: Option<String>,

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

    // The splits that make up the transaction.  The sum of these splits must
    // always be balanced.
    //    pub splits: Vec<std::rc::Weak<Split>>,
    pub splits: Vec<Split>,
}

#[derive(Debug)]
pub enum Quantity {
    // The amount of the transaction, as seen on the bank statement.
    // This could be a number of shares when the account is a Stock account, for
    // instance, or a number of EUR for a checking account.
    Credit(Value),

    // Buying shares
    Buy(Value),

    // Reinvest dividends and buy shares
    Reinvest(Value),

    // There were some dividends for one of the stocks   The amount will be
    // visible in other splits.
    Dividend(Value),

    // Used for stock splits.  The number of shares is multiplied by the ratio,
    // and their value divided by the same ratio.
    Split {
        ratio: Decimal,
        commodity: CommodityId,
    },
}

/// GnuCash and Kmymoney call these splits.
/// Beancount and Ledger call these postings.
#[derive(Debug)]
pub struct Split {
    // The transaction this belongs to
    //    transaction: std::rc::Rc<Transaction>,

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
    pub original_value: Quantity,

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
    pub value: Option<Quantity>,
}
