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
#[derive(Default)]
pub struct Metrics {
    // Networth at start and end of the period
    pub start_networth: MultiValue,
    pub end_networth: MultiValue,

    // Networth = networth_liquid + networth_illiquid
    // Only the liquid part (cash, investments,...) of the networth
    pub start_networth_liquid: MultiValue,
    pub end_networth_liquid: MultiValue,

    // The illiquid part (real-estate,...)
    pub start_networth_illiquid: MultiValue,
    pub end_networth_illiquid: MultiValue,

    // Total realized income (salaries, passive, rents,...).
    // Note: this is generally a negative value (as it leaves a category to go
    // INTO a user account)
    pub income: MultiValue,

    // Only the passive part of the income, e.g. dividends, interests paid,
    // rents received, ...
    // This is generally a negative value, see above
    pub passive_income: MultiValue,

    // Only the part of the income that comes from work
    pub work_income: MultiValue,

    // Total expenses
    // Note: this is generally a positive value (as it goes FROM user accounts)
    pub expense: MultiValue,

    // P&L = networth_at_end - networth_at_start = pnl_liquid + pnl_illiquid
    // The total variation of the networth.
    pub pnl: MultiValue,
    pub pnl_liquid: MultiValue,
    pub pnl_illiquid: MultiValue,

    // Cashflow = income - expense
    // This is the total amount of money added to the networth (i.e. to any of
    // the user accounts) via actual money transfers.
    pub cashflow: MultiValue,

    // Unrealized = P&L - cashflow
    // This is the variation in market value (prices and exchange rates) of
    // investments.
    pub unrealized: MultiValue,
    pub unrealized_liquid: MultiValue,
    pub unrealized_illiquid: MultiValue,

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

    // Return-on-Investment =
    //    (passive_income + unrealized + pnl_illiquid) / networth
    pub roi: Option<Decimal>,

    // Return-on-Investment for liquid assets =
    //    (passive_income + unrealized_liquid) / networth_liquid
    pub roi_liquid: Option<Decimal>,

    // How many days of expenses can be funded through liquid assets
    pub emergency_fund: Option<Decimal>,

    // How many days of expenses you own
    pub wealth: Option<Decimal>,

    // How much income tax rate
    pub income_tax: MultiValue,
    pub misc_tax: MultiValue,
    pub income_tax_rate: Option<Decimal>,
}

impl Metrics {
    /// Compute various statistics over a range of time
    pub fn new(
        repo: &Repository,
        settings: Settings,
        now: DateTime<Local>,
    ) -> Result<Self> {
        let ts_range = &settings.over.to_ranges(now)?[0];
        let mut stats = Metrics::default();
        let mut prices = repo.market_prices(settings.commodity.clone());
        let mut start_prices = repo.market_prices(settings.commodity.clone());
        let mut end_prices = repo.market_prices(settings.commodity.clone());
        let mut start_value_liquid = MultiValue::zero();
        let mut end_value_liquid = MultiValue::zero();
        let mut start_value_illiquid = MultiValue::zero();
        let mut end_value_illiquid = MultiValue::zero();

        for tx in repo.transactions.iter() {
            for s in tx.splits().iter() {
                let kind = s.account.get_kind();
                if kind.is_unrealized() {
                    // nothing to do
                } else if kind.is_expense()
                    || kind.is_income()
                    || kind.is_passive_income()
                {
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

                    if ts_range.intv.contains(s.post_ts) {
                        if kind.is_income_tax() {
                            stats.income_tax += &val;
                        } else if kind.is_misc_tax() {
                            stats.misc_tax += &val;
                        }

                        if kind.is_expense() {
                            stats.expense += &val;
                        } else if kind.is_passive_income() {
                            stats.passive_income += &val;
                            stats.income += &val;
                        } else if kind.is_work_income() {
                            stats.work_income += &val;
                            stats.income += &val;
                        } else {
                            stats.income += &val;
                        }
                    }
                } else if kind.is_networth() {
                    // An operation before the start of the time range:
                    // this is used to compute the starting state
                    if ts_range.intv.strictly_right_of(s.post_ts) {
                        if kind.is_liquid() {
                            start_value_liquid.apply(&s.operation);
                        } else {
                            start_value_illiquid.apply(&s.operation);
                        }
                    }

                    // An operation before the end of the time range:
                    // this is used to compute the ending state.
                    if !ts_range.intv.strictly_left_of(s.post_ts) {
                        if kind.is_liquid() {
                            end_value_liquid.apply(&s.operation);
                        } else {
                            end_value_illiquid.apply(&s.operation);
                        }
                    }
                }
            }
        }

        stats.start_networth_liquid = start_prices.convert_multi_value(
            &start_value_liquid,
            ts_range.intv.lower().expect("bounded interval"),
        );
        stats.start_networth_illiquid = start_prices.convert_multi_value(
            &start_value_illiquid,
            ts_range.intv.lower().expect("bounded interval"),
        );
        stats.end_networth_liquid = start_prices.convert_multi_value(
            &end_value_liquid,
            ts_range.intv.upper().expect("bounded interval"),
        );
        stats.end_networth_illiquid = start_prices.convert_multi_value(
            &end_value_illiquid,
            ts_range.intv.upper().expect("bounded interval"),
        );
        stats.start_networth =
            &stats.start_networth_liquid + &stats.start_networth_illiquid;
        stats.end_networth =
            &stats.end_networth_liquid + &stats.end_networth_illiquid;
        stats.pnl_liquid =
            &stats.end_networth_liquid - &stats.start_networth_liquid;
        stats.pnl_illiquid =
            &stats.end_networth_illiquid - &stats.start_networth_illiquid;
        stats.pnl = &stats.end_networth - &stats.start_networth;
        stats.cashflow = &stats.income + &stats.expense;
        stats.unrealized = &stats.pnl + &stats.cashflow;
        stats.unrealized_liquid = &stats.pnl_liquid + &stats.cashflow;
        stats.unrealized_illiquid = stats.pnl_illiquid.clone();
        stats.saving_rate = &stats.cashflow / &stats.income;
        stats.financial_independence =
            (&stats.unrealized - &stats.passive_income) / &stats.expense;
        stats.passive_income_ratio =
            (&stats.passive_income - &stats.unrealized) / &stats.income;
        stats.roi =
            (&stats.passive_income + &stats.unrealized + &stats.pnl_illiquid)
                / &stats.start_networth;
        stats.roi_liquid = (&stats.passive_income + &stats.unrealized_liquid)
            / &stats.start_networth_liquid;

        let days = ts_range.duration(now).num_days();
        let daily_expense = &stats.expense / Decimal::from(days);
        stats.emergency_fund = &stats.end_networth_liquid / &daily_expense;
        stats.wealth = &stats.end_networth / &daily_expense;

        let mkt_income_tax = end_prices.convert_multi_value(
            &stats.income_tax,
            ts_range.intv.upper().expect("bounded interval"),
        );
        stats.income_tax_rate = &mkt_income_tax / -&stats.income;

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
