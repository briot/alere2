use crate::{
    commodities::Commodity,
    multi_values::{MultiValue, Operation},
    repositories::Repository,
    times::Intv,
};
use anyhow::Result;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;

pub struct Settings {
    pub commodity: Option<Commodity>,

    // What columns to display.  Each column aggregates all transaction within
    // a time interval.
    pub over: Intv,
}

/// Changes in one time range
#[derive(Debug, Default)]
pub struct PerAccount {
    pub start_value: MultiValue,
    pub end_value: MultiValue,
}

#[derive(Default)]
pub struct Stats {
    // Networth at start and end of the period
    pub start_networth: MultiValue,
    pub end_networth: MultiValue,

    // Total realized income (salaries, passive, rents,...).
    // Note: this is generally a negative value (as it leaves a category to go
    // INTO a user account)
    pub income: MultiValue,

    // Only the passive part of the income, e.g. dividends, interests paid,
    // rents received, ...
    // This is generally a negative value, see above
    pub passive_income: MultiValue,

    // Total expenses
    // Note: this is generally a positive value (as it goes FROM user accounts)
    pub expense: MultiValue,

    // P&L = networth_at_end - networth_at_start
    // The total variation of the networth.
    pub pnl: MultiValue,

    // Cashflow = income - expense
    // This is the total amount of money added to the networth (i.e. to any of
    // the user accounts) via actual money transfers.
    pub cashflow: MultiValue,

    // Unrealized = P&L - cashflow
    // This is the variation in market value (prices and exchange rates) of
    // investments.
    pub unrealized: MultiValue,

    // Saving rate = 1 - Expense / Income
    // How much of the income we are saving (i.e. they remain in accounts)
    pub saving_rate: Option<Decimal>,

    // Financial independence = (passive_income + unrealized_income) / expenses
    // How much our expenses are covered by passive income.  I.e. would we be
    // able to cover those expenses if we stopped getting a salary.
    pub financial_independence: Option<Decimal>,

    // Passive income ratio = (passive_income + unrealized) / income
    // What part of total income comes from sources other that salaries
    pub passive_income_ratio: Option<Decimal>,
}

impl Stats {
    /// Compute various statistics over a range of time
    pub fn new(
        repo: &Repository,
        settings: Settings,
        now: DateTime<Local>,
    ) -> Result<Self> {
        let ts_range = &settings.over.to_ranges(now)?[0].intv;
        let mut stats = Stats::default();
        let mut prices = repo.market_prices(settings.commodity.clone());
        let mut start_prices = repo.market_prices(settings.commodity.clone());
        let mut end_prices = repo.market_prices(settings.commodity.clone());

        repo.accounts.iter().for_each(|acc| {
            let kind = &acc.get_kind();
            let mut per_account = PerAccount::default();

            // True if this transaction is about unrealized gain.  This
            // is always true if our account itself is unrealized
            let mut tx_is_unrealized = kind.is_unrealized();

            acc.iter_transactions().for_each(|tx| {
                // The sum of splits from networth account.  For instance,
                // this would be money transferred from one account to an
                // other to buy shares (but if there are bank fees those
                // are counted, while just looking at the amount to buy
                // shares would not include them).
                // These also do not include acc itself.
                let mut internal_flow = MultiValue::zero();
                let mut external_flow = MultiValue::zero();

                for s in tx.splits().iter() {
                    if s.account == acc {
                        // An operation before the start of the time range:
                        // this is used to compute the starting state
                        if ts_range.strictly_right_of(s.post_ts) {
                            per_account.start_value.apply(&s.operation);
                        }

                        // An operation before the end of the time range:
                        // this is used to compute the ending state.
                        if !ts_range.strictly_left_of(s.post_ts) {
                            per_account.end_value.apply(&s.operation);
                        }
                    } else {
                        let k = s.account.get_kind();
                        tx_is_unrealized |= k.is_unrealized();

                        let val = match &s.operation {
                            Operation::Credit(v) => {
                                prices.convert_multi_value(v, &s.post_ts)
                            }
                            Operation::AddShares { qty } => {
                                prices.convert_value(qty, &s.post_ts)
                            }
                            Operation::BuyAmount { .. }
                            | Operation::BuyPrice { .. }
                            | Operation::Reinvest { .. }
                            | Operation::Split { .. }
                            | Operation::Dividend => MultiValue::zero(),
                        };

                        if k.is_networth() {
                            internal_flow += val;
                        } else {
                            external_flow += val;
                        }
                    }
                }
            });

            let mkt_start_value = start_prices.convert_multi_value(
                &per_account.start_value,
                ts_range.lower().expect("bounded interval"),
            );
            let mkt_end_value = end_prices.convert_multi_value(
                &per_account.end_value,
                ts_range.upper().expect("bounded interval"),
            );
            let pnl = &mkt_end_value - &mkt_start_value;

            if tx_is_unrealized {
            } else if kind.is_expense() {
                stats.expense += &pnl;
            } else if kind.is_passive_income() {
                stats.income += &pnl;
                stats.passive_income += &pnl;
            } else if kind.is_income() {
                stats.income += &pnl;
            }

            if kind.is_networth() {
                stats.start_networth += mkt_start_value;
                stats.end_networth += mkt_end_value;
            }
        });

        stats.pnl = &stats.end_networth - &stats.start_networth;
        stats.cashflow = &stats.income + &stats.expense;
        stats.unrealized = &stats.pnl + &stats.cashflow;
        stats.saving_rate = &stats.cashflow / &stats.income;
        stats.financial_independence =
            (&stats.unrealized - &stats.passive_income) / &stats.expense;
        stats.passive_income_ratio =
            (&stats.passive_income - &stats.unrealized) / &stats.income;
        Ok(stats)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        account_categories::AccountCategory,
        account_kinds::AccountKind,
        accounts::{Account, AccountCollection},
        commodities::CommodityCollection,
        multi_values::{MultiValue, Operation, Value},
        transactions::{ReconcileKind, Transaction},
    };
    use chrono::prelude::*;
    use rust_decimal_macros::dec;

    fn build_tx(
        ts: DateTime<Local>,
        splits: Vec<(Account, Operation)>,
    ) -> Transaction {
        let mut tr = Transaction::new_with_default();
        for s in splits.into_iter() {
            tr.add_split(s.0, ReconcileKind::New, ts, s.1);
        }
        assert!(tr.is_balanced());
        tr
    }

    #[test]
    fn test_stats() {
        let mut coms = CommodityCollection::default();
        let mut accounts = AccountCollection::default();
        let kind =
            AccountKind::new("eee", "Inc", "Dec", AccountCategory::EXPENSE);
        let acc_cash = accounts.add_dummy("cash", kind.clone());
        let acc_invest = accounts.add_dummy("invest", kind.clone());
        let acc_fees = accounts.add_dummy("fees", kind.clone());
        let comm_eur = coms.add_dummy("eur", true);
        let comm_share = coms.add_dummy("shares", false);

        let _tr = build_tx(
            Local.with_ymd_and_hms(2024, 5, 24, 12, 0, 0).unwrap(),
            vec![
                (
                    acc_cash,
                    Operation::Credit(MultiValue::new(
                        dec!(-2544.66),
                        &comm_eur,
                    )),
                ),
                (
                    acc_fees,
                    Operation::Credit(MultiValue::new(dec!(12.66), &comm_eur)),
                ),
                (
                    acc_invest,
                    Operation::BuyPrice {
                        qty: Value {
                            amount: dec!(3),
                            commodity: comm_share,
                        },
                        price: Value {
                            amount: dec!(844),
                            commodity: comm_eur.clone(),
                        },
                    },
                ),
            ],
        );
    }
}
