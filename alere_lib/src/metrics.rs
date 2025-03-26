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
        let mut income_tax = MultiValue::zero();

        repo.accounts.iter().for_each(|acc| {
            let kind = &acc.get_kind();
            let mut start_value = MultiValue::zero();
            let mut end_value = MultiValue::zero();

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
                        if ts_range.intv.strictly_right_of(s.post_ts) {
                            start_value.apply(&s.operation);
                        }

                        // An operation before the end of the time range:
                        // this is used to compute the ending state.
                        if !ts_range.intv.strictly_left_of(s.post_ts) {
                            end_value.apply(&s.operation);
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

                        // Will be computed twice if a transaction has more
                        // than two splits.
                        if k.is_income_tax()
                            && ts_range.intv.contains(s.post_ts)
                        {
                            println!("MANU income tax={:?} {} {:?}",
                                s.post_ts,
                                s.account.name(AccountNameDepth::unlimited()),
                                val.display(&Formatter::default()));
                            income_tax += &val;
                        }

                        if k.is_networth() {
                            internal_flow += &val;
                        } else {
                            external_flow += &val;
                        }
                    }
                }
            });

            let mkt_start_value = start_prices.convert_multi_value(
                &start_value,
                ts_range.intv.lower().expect("bounded interval"),
            );
            let mkt_end_value = end_prices.convert_multi_value(
                &end_value,
                ts_range.intv.upper().expect("bounded interval"),
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
                if kind.is_liquid() {
                    stats.start_networth_liquid += mkt_start_value;
                    stats.end_networth_liquid += mkt_end_value;
                } else {
                    stats.start_networth_illiquid += mkt_start_value;
                    stats.end_networth_illiquid += mkt_end_value;
                }
            }
        });

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
            &income_tax,
            ts_range.intv.upper().expect("bounded interval"),
        );
        stats.income_tax_rate = &mkt_income_tax / -&stats.income;
        println!(
            "MANU income_tax={} {}",
            income_tax.display(&Formatter::default()),
            mkt_income_tax.display(&Formatter::default())
        );

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
