use crate::{
    accounts::AccountNameDepth,
    commodities::Commodity,
    formatters::Formatter,
    multi_values::{MultiValue, Operation},
    repositories::Repository,
    times::Intv,
};
use anyhow::Result;
use chrono::{DateTime, Local};

pub struct Settings {
    pub commodity: Option<Commodity>,

    // What columns to display.  Each column aggregates all transaction within
    // a time interval.
    pub over: Intv,
}

/// Changes in one time range
#[derive(Default)]
pub struct PerAccount {
    pub start_value: MultiValue,
    pub mkt_start_value: MultiValue,
    pub end_value: MultiValue,
    pub mkt_end_value: MultiValue,

    // Total change in market value
    pub pnl_no_dividends: MultiValue,

    // Gains from explicit user operations (credit, buying shares,...)
    // This is computed in the settings' commodity at the time of the operation.
    pub mkt_cashflow: MultiValue,

    // Market value change from changes in prices and xrates.
    // We have the invariants:
    //     pnl = mkt_end_value - mkt_start_value
    //     pnl = mkt_cashflow + mkt_unrealized
    pub mkt_unrealized: MultiValue,
}

#[derive(Default)]
pub struct Stats {
    pub mkt_income: MultiValue,
    pub mkt_expense: MultiValue,
    pub mkt_passive_income: MultiValue,
    pub mkt_unrealized: MultiValue,
    pub mkt_start_networth: MultiValue,
    pub mkt_end_networth: MultiValue,
}

impl Stats {
    /// Compute various statistics over a range of time
    pub fn new(
        repo: &Repository,
        settings: Settings,
        now: DateTime<Local>,
        format: &Formatter,
    ) -> Result<Self> {
        let ts_range = &settings.over.to_ranges(now)?[0].intv;
        let mut stats = Stats::default();
        let mut prices = repo.market_prices(settings.commodity.clone());
        let mut start_prices = repo.market_prices(settings.commodity.clone());
        let mut end_prices = repo.market_prices(settings.commodity.clone());

        repo.accounts.iter()
            .for_each(|acc| {
                let kind = &acc.get_kind();
                let mut per_account = PerAccount::default();

                acc.iter_transactions().for_each(|tx| {
                    // The sum of splits from networth account.  For instance,
                    // this would be money transferred from one account to an
                    // other to buy shares (but if there are bank fees those
                    // are counted, while just looking at the amount to buy
                    // shares would not include them).
                    // These also do not include acc itself.
                    let mut internal_flow = MultiValue::zero();
                    let mut external_flow = MultiValue::zero();

                    // True if this transaction is about unrealized gain.  This
                    // is always true if our account itself is unrealized
                    let mut tx_is_unrealized = kind.is_unrealized();

                    for s in tx.iter_splits() {
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

                        println!("MANU tx={:?}\n   internal={:?}\n   external={:?}\n   unrealized={:?}", tx, internal_flow, external_flow, tx_is_unrealized);

                        if !tx_is_unrealized {
                            per_account.mkt_cashflow -= &internal_flow;
                        }
                    }
                });

                per_account.mkt_start_value = start_prices.convert_multi_value(
                    &per_account.start_value,
                    ts_range.lower()
                       .expect("bounded interval"),
                );
                per_account.mkt_end_value = end_prices.convert_multi_value(
                    &per_account.end_value,
                    ts_range.upper()
                       .expect("bounded interval"),
                );
                per_account.pnl_no_dividends =
                    &per_account.mkt_end_value - &per_account.mkt_start_value;
                per_account.mkt_unrealized =
                    &per_account.pnl_no_dividends - &per_account.mkt_cashflow;

                if !per_account.mkt_unrealized.is_zero() {
                    println!(
                        "MANU {} pnl_no_dividends={} = cashflow={} + unrealized={}",
                        acc.name(AccountNameDepth::basename()),
                        per_account.pnl_no_dividends.display(format),
                        per_account.mkt_cashflow.display(format),
                        per_account.mkt_unrealized.display(format),
                    );
                }
                println!(
                   "MANU {} start={} end={}",
                   acc.name(AccountNameDepth::basename()),
                    per_account.mkt_start_value.display(format),
                    per_account.mkt_end_value.display(format));

                if kind.is_unrealized() {
                    //  stats.mkt_unrealized += per_account.pnl;
                } else if kind.is_expense() {
                    stats.mkt_expense += &per_account.pnl_no_dividends;
                } else if kind.is_income() {
                    stats.mkt_income += &per_account.pnl_no_dividends;

                    if kind.is_passive_income() {
                        stats.mkt_passive_income += &per_account.pnl_no_dividends;
                    }
                }

                // if !tx_is_unrealized {
                //     per_account.mkt_cashflow -= internal_flow;
                // }

                per_account.mkt_start_value = start_prices.convert_multi_value(
                    &per_account.start_value,
                    ts_range.lower().expect("bounded interval"),
                );
                per_account.mkt_end_value = end_prices.convert_multi_value(
                    &per_account.end_value,
                    ts_range.upper().expect("bounded interval"),
                );
                per_account.pnl_no_dividends =
                    &per_account.mkt_end_value - &per_account.mkt_start_value;
                per_account.mkt_unrealized =
                    &per_account.pnl_no_dividends - &per_account.mkt_cashflow;

                if kind.is_unrealized() {
                    //  stats.mkt_unrealized += per_account.pnl;
                } else if kind.is_expense() {
                    stats.mkt_expense += &per_account.pnl_no_dividends;
                } else if kind.is_income() {
                    stats.mkt_income += &per_account.pnl_no_dividends;

                    if kind.is_passive_income() {
                        stats.mkt_passive_income += &per_account.pnl_no_dividends;
                    }
                }
                if kind.is_networth() {
                    stats.mkt_start_networth += per_account.mkt_start_value;
                    stats.mkt_end_networth += per_account.mkt_end_value;
                }
                stats.mkt_unrealized += per_account.mkt_unrealized;
        });

        //        stats.start.mkt_all_income = start_prices
        //            .convert_multi_value(&stats.start.all_income, &ts_range.lower());
        //        stats.start.mkt_all_expense = start_prices
        //            .convert_multi_value(&stats.start.all_expense, &ts_range.lower());
        //        stats.start.mkt_networth = start_prices
        //            .convert_multi_value(&stats.start.networth, &ts_range.lower());
        //        stats.start.mkt_passive_income = start_prices.convert_multi_value(
        //            &stats.start.passive_income,
        //            &ts_range.lower(),
        //        );
        //        stats.start.mkt_unrealized = start_prices
        //            .convert_multi_value(&stats.start.unrealized, &ts_range.lower());
        //
        //        stats.end.mkt_all_income = end_prices
        //            .convert_multi_value(&stats.end.all_income, &ts_range.upper());
        //        stats.end.mkt_all_expense = end_prices
        //            .convert_multi_value(&stats.end.all_expense, &ts_range.upper());
        //        stats.end.mkt_networth = end_prices
        //            .convert_multi_value(&stats.end.networth, &ts_range.upper());
        //        stats.end.mkt_passive_income = end_prices
        //            .convert_multi_value(&stats.end.passive_income, &ts_range.upper());
        //        stats.end.mkt_unrealized = end_prices
        //            .convert_multi_value(&stats.end.unrealized, &ts_range.upper());

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
        transactions::{ReconcileKind, TransactionRc},
    };
    use chrono::prelude::*;
    use rust_decimal_macros::dec;

    fn build_tx(
        ts: DateTime<Local>,
        splits: Vec<(Account, Operation)>,
    ) -> TransactionRc {
        let mut tr = TransactionRc::new_with_default();
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
