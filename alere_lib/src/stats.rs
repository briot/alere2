use crate::commodities::CommodityId;
use crate::multi_values::{MultiValue, Operation, Value};
use crate::repositories::Repository;
use crate::times::Interval;
use anyhow::Result;
use chrono::{DateTime, Local};

pub struct Settings {
    pub commodity: Option<CommodityId>,

    // What columns to display.  Each column aggregates all transaction within
    // a time interval.
    pub over: Interval,
}

/// Changes in one time range
#[derive(Default)]
pub struct PerAccount {
    pub start_value: MultiValue,
    pub mkt_start_value: MultiValue,
    pub end_value: MultiValue,
    pub mkt_end_value: MultiValue,

    // Total change in market value
    pub pnl: MultiValue,

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
    ) -> Result<Self> {
        let ts_range = &settings.over.to_ranges(now)?[0].intv;
        let mut stats = Stats::default();
        let mut start_prices = repo.market_prices(settings.commodity);
        let mut end_prices = repo.market_prices(settings.commodity);

        repo.iter_accounts()
//            .filter(|(_acc_id, acc)| acc.name == "Air liquide")
            .for_each(|(acc_id, acc)| {
                let kind = repo.account_kinds.get(acc.kind).unwrap();
                let mut per_account = PerAccount::default();

                acc.iter_splits(acc_id).for_each(|s| {
                    // An operation before the start of the time range: this is
                    // used to compute the starting state
                    if ts_range.strictly_right_of(&s.post_ts) {
                        per_account.start_value.apply(&s.operation);
                    }

                    // An operation before the end of the time range: this is
                    // used to compute the ending state.
                    if !ts_range.strictly_left_of(&s.post_ts) {
                        per_account.end_value.apply(&s.operation);
                    }

                    if ts_range.contains(&s.post_ts) {
                        match &s.operation {
                            Operation::Credit(value) => {
                                // ??? Should not add if this is coming from
                                // a "unrealized" account
                                per_account.mkt_cashflow += value;
                            }
                            Operation::AddShares { .. }
                            | Operation::Reinvest { .. }
                            | Operation::Dividend
                            | Operation::Split { .. } => {}
                            Operation::BuyAmount { amount, .. } => {
                                per_account.mkt_cashflow += amount;
                            }
                            Operation::BuyPrice { qty, price } => {
                                per_account.mkt_cashflow += Value {
                                    amount: qty.amount * price.amount,
                                    commodity: price.commodity,
                                };
                            }
                        }
                    }
                });

                per_account.mkt_start_value = start_prices.convert_multi_value(
                    &per_account.start_value,
                    &ts_range.lower(),
                );
                per_account.mkt_end_value = end_prices.convert_multi_value(
                    &per_account.end_value,
                    &ts_range.upper(),
                );
                per_account.pnl = 
                    &per_account.mkt_end_value - &per_account.mkt_start_value;
                per_account.mkt_unrealized = 
                    &per_account.pnl - &per_account.mkt_cashflow;

                if !per_account.mkt_unrealized.is_zero() {
                    println!(
                        "MANU {} pnl={} = cashflow={} + unrealized={}",
                        acc.name,
                        repo.display_multi_value(&per_account.pnl),
                        repo.display_multi_value(&per_account.mkt_cashflow),
                        repo.display_multi_value(&per_account.mkt_unrealized),
                    );
                }

                if kind.is_unrealized {
                    //  stats.mkt_unrealized += per_account.pnl;
                } else if kind.is_expense() {
                    stats.mkt_expense += &per_account.pnl;
                } else if kind.is_income() {
                    stats.mkt_income += &per_account.pnl;

                    if kind.is_passive_income {
                        stats.mkt_passive_income += &per_account.pnl;
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
