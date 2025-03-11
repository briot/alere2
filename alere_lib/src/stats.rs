use crate::formatters::Formatter;
use crate::commodities::CommodityId;
use crate::multi_values::{MultiValue, Operation};
use crate::repositories::Repository;
use crate::times::Intv;
use anyhow::Result;
use chrono::{DateTime, Local};

pub struct Settings {
    pub commodity: Option<CommodityId>,

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
        let mut prices = repo.market_prices(settings.commodity);
        let mut start_prices = repo.market_prices(settings.commodity);
        let mut end_prices = repo.market_prices(settings.commodity);

        repo.iter_accounts()
            .filter(|(_acc_id, acc)| acc.name == "ASML HOLDING"
                || acc.name == "Actions AdaCore")
            .for_each(|(acc_id, acc)| {
                let kind = repo.account_kinds.get(acc.kind).unwrap();
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
                    let mut tx_is_unrealized = kind.is_unrealized;

                    for s in tx.iter_splits() {
                        if s.account == acc_id {
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
                            let s_acc = repo.get_account(s.account).unwrap();
                            let k = repo.account_kinds.get(s_acc.kind).unwrap();

                            tx_is_unrealized |= k.is_unrealized;

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

                            if k.is_networth {
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
                        acc.name,
                        repo.display_multi_value(&per_account.pnl_no_dividends, format),
                        repo.display_multi_value(&per_account.mkt_cashflow, format),
                        repo.display_multi_value(&per_account.mkt_unrealized, format),
                    );
                }
                println!(
                   "MANU {} start={} end={}",
                   acc.name,
                   repo.display_multi_value(&per_account.mkt_start_value, format),
                   repo.display_multi_value(&per_account.mkt_end_value, format));

                if kind.is_unrealized {
                    //  stats.mkt_unrealized += per_account.pnl;
                } else if kind.is_expense() {
                    stats.mkt_expense += &per_account.pnl_no_dividends;
                } else if kind.is_income() {
                    stats.mkt_income += &per_account.pnl_no_dividends;

                    if kind.is_passive_income {
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

                if kind.is_unrealized {
                    //  stats.mkt_unrealized += per_account.pnl;
                } else if kind.is_expense() {
                    stats.mkt_expense += &per_account.pnl_no_dividends;
                } else if kind.is_income() {
                    stats.mkt_income += &per_account.pnl_no_dividends;

                    if kind.is_passive_income {
                        stats.mkt_passive_income += &per_account.pnl_no_dividends;
                    }
                }
                if kind.is_networth {
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
    use crate::accounts::AccountId;
    use crate::commodities::CommodityId;
    use crate::multi_values::{MultiValue, Operation, Value};
    use crate::transactions::{ReconcileKind, TransactionRc};
    use chrono::prelude::*;
    use rust_decimal_macros::dec;

    fn build_tx(
        ts: DateTime<Local>,
        splits: Vec<(AccountId, Operation)>,
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
        let acc_cash = AccountId(1);
        let acc_invest = AccountId(2);
        let acc_fees = AccountId(3);

        let comm_eur = CommodityId(1);
        let comm_share = CommodityId(2);

        let _tr = build_tx(
            Local.with_ymd_and_hms(2024, 5, 24, 12, 0, 0).unwrap(),
            vec![
                (
                    acc_cash,
                    Operation::Credit(MultiValue::new(
                        dec!(-2544.66),
                        comm_eur,
                    )),
                ),
                (
                    acc_fees,
                    Operation::Credit(MultiValue::new(dec!(12.66), comm_eur)),
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
                            commodity: comm_eur,
                        },
                    },
                ),
            ],
        );
    }
}
