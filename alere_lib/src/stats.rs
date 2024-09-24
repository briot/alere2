use crate::commodities::CommodityId;
use crate::multi_values::MultiValue;
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

// #[derive(Default)]
// struct AccountSummary {
//     pub balance: Value,
//     //  Balance of this account (number of shares for instance)
//
//     pub invested: Value,
//     //  How much we invested to buy those shares.
// }
//
// impl AccountSummary {
//     fn apply_split(&mut self, split: &Split) {
//         self.balance.apply(&split.original_value);
//
//         match split.original_value {
//             Operation::Buy(_shares) => {
//                 if let Some(v) = split.value {
//                     self.invested += &v;
//                 }
//             }
//             Operation::Credit(_value)  => {}
//             Operation::Reinvest(_shares) => {}
//             Operation::Split { ratio: _, commodity: _ } => {}
//             Operation::Dividend(_value) => { }
//         }
//     }
// }

#[derive(Default)]
pub struct StatsTick {
    pub all_income: MultiValue,     // Sign is negative
    pub mkt_all_income: MultiValue, // Sign is negative

    pub passive_income: MultiValue, // Sign is negative
    pub mkt_passive_income: MultiValue, // Sign is negative

    pub unrealized: MultiValue,     // Sign is negative
    pub mkt_unrealized: MultiValue, // Sign is negative

    pub all_expense: MultiValue,
    pub mkt_all_expense: MultiValue,

    pub networth: MultiValue,
    pub mkt_networth: MultiValue,
}

#[derive(Default)]
pub struct Stats {
    pub start: StatsTick,
    pub end: StatsTick,
}

pub struct MultiStats(pub Vec<Stats>);

impl Stats {
    /// Compute various statistics over a range of time
    pub fn new(
        repo: &Repository,
        settings: Settings,
        now: DateTime<Local>,
    ) -> Result<Self> {
        let ts_range = settings.over.to_ranges(now)?;
        let ts_range = ts_range[0].intv.clone();
        let mut stats = Stats::default();
        let mut start_prices = repo.market_prices(settings.commodity);
        let mut end_prices = repo.market_prices(settings.commodity);

        repo.iter_accounts().for_each(|(acc_id, acc)| {
            let kind = repo.account_kinds.get(acc.kind).unwrap();
            let mut cumul_start = MultiValue::default();
            let mut cumul_end = MultiValue::default();

            acc.iter_splits(acc_id).for_each(|s| {
                if ts_range.strictly_right_of(&s.post_ts) {
                    cumul_start.apply(&s.operation);
                }
                if !ts_range.strictly_left_of(&s.post_ts) {
                    cumul_end.apply(&s.operation);
                }
            });

            if kind.is_expense() && !kind.is_unrealized {
                stats.start.all_expense += &cumul_start;
                stats.end.all_expense += &cumul_end;
            }
            if kind.is_income() && !kind.is_unrealized {
                stats.start.all_income += &cumul_start;
                stats.end.all_income += &cumul_end;
            }
            if kind.is_passive_income {
                stats.start.passive_income += &cumul_start;
                stats.end.passive_income += &cumul_end;
            }
            if kind.is_unrealized {
                stats.start.unrealized += &cumul_start;
                stats.end.unrealized += &cumul_end;
            }
            if kind.is_networth {
                stats.start.networth += &cumul_start;
                stats.end.networth += &cumul_end;
            }
        });

        stats.start.mkt_all_income = start_prices
            .convert_multi_value(&stats.start.all_income, &ts_range.lower());
        stats.start.mkt_all_expense = start_prices
            .convert_multi_value(&stats.start.all_expense, &ts_range.lower());
        stats.start.mkt_networth = start_prices
            .convert_multi_value(&stats.start.networth, &ts_range.lower());
        stats.start.mkt_passive_income = start_prices.convert_multi_value(
            &stats.start.passive_income,
            &ts_range.lower(),
        );
        stats.start.mkt_unrealized = start_prices
            .convert_multi_value(&stats.start.unrealized, &ts_range.lower());

        stats.end.mkt_all_income = end_prices
            .convert_multi_value(&stats.end.all_income, &ts_range.upper());
        stats.end.mkt_all_expense = end_prices
            .convert_multi_value(&stats.end.all_expense, &ts_range.upper());
        stats.end.mkt_networth = end_prices
            .convert_multi_value(&stats.end.networth, &ts_range.upper());
        stats.end.mkt_passive_income = end_prices
            .convert_multi_value(&stats.end.passive_income, &ts_range.upper());
        stats.end.mkt_unrealized = end_prices
            .convert_multi_value(&stats.end.unrealized, &ts_range.upper());

        Ok(stats)
    }
}
