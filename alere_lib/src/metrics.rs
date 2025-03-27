use crate::{
    commodities::Commodity,
    market_prices::MarketPrices,
    multi_values::{MultiValue, Operation},
    repositories::Repository,
    times::{Intv, TimeInterval},
};
use anyhow::Result;
use chrono::{DateTime, Local};
use itertools::Itertools;
use rust_decimal::Decimal;

pub struct Settings {
    pub commodity: Option<Commodity>,

    // What columns to display.  Each column aggregates all transaction within
    // a time interval.
    pub intervals: Vec<Intv>,
}

/// Changes in one time range
pub struct Metrics {
    pub interval: TimeInterval,

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

#[derive(Default)]
struct MetricsArgs {
    start_liquid: MultiValue,
    start_illiquid: MultiValue,
    end_liquid: MultiValue,
    end_illiquid: MultiValue,
    income: MultiValue,
    passive_income: MultiValue,
    work_income: MultiValue,
    expense: MultiValue,
    income_tax: MultiValue,
    misc_tax: MultiValue,
}

impl Metrics {
    fn new(
        prices: &mut MarketPrices,
        now: DateTime<Local>,
        args: MetricsArgs,
        interval: TimeInterval,
    ) -> Self {
        let lo = interval.intv.lower().expect("bounded interval");
        let up = interval.intv.upper().expect("bounded interval");
        let start_liquid = prices.convert_multi_value(&args.start_liquid, lo);
        let start_illiquid =
            prices.convert_multi_value(&args.start_illiquid, lo);
        let end_liquid = prices.convert_multi_value(&args.end_liquid, up);
        let end_illiquid = prices.convert_multi_value(&args.end_illiquid, up);
        let income_tax = prices.convert_multi_value(&args.income_tax, up);
        let cashflow = &args.income + &args.expense;
        let start_nw = &start_liquid + &start_illiquid;
        let end_nw = &end_liquid + &end_illiquid;
        let pnl = &end_nw - &start_nw;
        let pnl_liquid = &end_liquid - &start_liquid;
        let pnl_illiquid = &end_illiquid - &start_illiquid;
        let unrealized = &pnl + &cashflow;
        let unrealized_liquid = &pnl_liquid + &cashflow;
        let days = interval.duration(now).num_days();
        let daily_expense = &args.expense / Decimal::from(days);
        Metrics {
            interval,
            unrealized_liquid: &pnl_liquid + &cashflow,
            saving_rate: &cashflow / &args.income,
            financial_independence: (&unrealized - &args.passive_income)
                / &args.expense,
            passive_income_ratio: (&args.passive_income - &unrealized)
                / &args.income,
            roi: (&args.passive_income + &unrealized + &pnl_illiquid)
                / &start_nw,
            roi_liquid: (&args.passive_income + unrealized_liquid)
                / &start_liquid,
            emergency_fund: &end_liquid / &daily_expense,
            wealth: &end_nw / &daily_expense,
            income_tax_rate: &income_tax / -&args.income,
            unrealized,
            unrealized_illiquid: pnl_illiquid.clone(),
            income_tax,
            misc_tax: args.misc_tax,
            start_networth_liquid: start_liquid,
            end_networth_liquid: end_liquid,
            start_networth_illiquid: start_illiquid,
            end_networth_illiquid: end_illiquid,
            start_networth: start_nw,
            end_networth: end_nw,
            income: args.income,
            passive_income: args.passive_income,
            work_income: args.work_income,
            expense: args.expense,
            pnl,
            pnl_liquid,
            pnl_illiquid,
            cashflow,
        }
    }

    /// Compute various statistics over a range of time
    pub fn load(
        repo: &Repository,
        settings: Settings,
        now: DateTime<Local>,
    ) -> Result<Vec<Self>> {
        let mut prices = repo.market_prices(settings.commodity.clone());
        let mut result = Vec::new();

        for result_stats in settings
            .intervals
            .iter()
            .map(|intv| intv.to_ranges(now))
            .flatten_ok()
        {
            let interval = match result_stats {
                Err(e) => Err(e)?,
                Ok(interval) => interval,
            };
            let mut args = MetricsArgs::default();

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

                        if interval.intv.contains(s.post_ts) {
                            if kind.is_income_tax() {
                                args.income_tax += &val;
                            } else if kind.is_misc_tax() {
                                args.misc_tax += &val;
                            }

                            if kind.is_expense() {
                                args.expense += &val;
                            } else if kind.is_passive_income() {
                                args.passive_income += &val;
                                args.income += &val;
                            } else if kind.is_work_income() {
                                args.work_income += &val;
                                args.income += &val;
                            } else {
                                args.income += &val;
                            }
                        }
                    } else if kind.is_networth() {
                        // An operation before the start of the time range:
                        // this is used to compute the starting state
                        if interval.intv.strictly_right_of(s.post_ts) {
                            if kind.is_liquid() {
                                args.start_liquid.apply(&s.operation);
                            } else {
                                args.start_illiquid.apply(&s.operation);
                            }
                        }

                        // An operation before the end of the time range:
                        // this is used to compute the ending state.
                        if !interval.intv.strictly_left_of(s.post_ts) {
                            if kind.is_liquid() {
                                args.end_liquid.apply(&s.operation);
                            } else {
                                args.end_illiquid.apply(&s.operation);
                            }
                        }
                    }
                }
            }

            result.push(Metrics::new(&mut prices, now, args, interval));
        }

        Ok(result)
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
