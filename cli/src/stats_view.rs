use crate::global_settings::GlobalSettings;
use alere_lib::{repositories::Repository, stats::Stats, times::Intv};
use anyhow::Result;
use rust_decimal::Decimal;

pub fn stats_view(
    repo: &Repository,
    globals: &GlobalSettings,
) -> Result<String> {
    let stats = Stats::new(
        repo,
        alere_lib::stats::Settings {
            commodity: globals.commodity.clone(),
            over: Intv::LastNYears(1),
        },
        globals.reftime,
        &globals.format,
    )?;
    let abs_income = -&stats.mkt_income;
    let abs_passive_income = -&stats.mkt_passive_income;
    let neg_expense = -&stats.mkt_expense;
    let mkt_cashflow = &abs_income + &neg_expense;

    Ok(format!(
        "
Networth:               {} to {}
Income:                 {} = passive {} + ...
Expenses:               {}
Cashflow:               {}
Unrealized:             + {}
Computed Market delta:  {} (networth delta {})
Savings Rate:           {:.1?}%
Financial Independence: {:.1?}%
Passive Income:         {:.1?}%
",
        stats.mkt_start_networth.display(&globals.format),
        stats.mkt_end_networth.display(&globals.format),
        abs_income.display(&globals.format),
        abs_passive_income.display(&globals.format),
        neg_expense.display(&globals.format),
        mkt_cashflow.display(&globals.format),
        stats.mkt_unrealized.display(&globals.format),
        (&mkt_cashflow + &stats.mkt_unrealized).display(&globals.format),
        (&stats.mkt_end_networth - &stats.mkt_start_networth)
            .display(&globals.format),
        (&mkt_cashflow / &abs_income).map(|p| p * Decimal::ONE_HUNDRED),
        (&(&abs_passive_income + &stats.mkt_unrealized) / &stats.mkt_expense)
            .map(|p| p * Decimal::ONE_HUNDRED),
        (&(&abs_passive_income + &stats.mkt_unrealized) / &abs_income)
            .map(|p| p * Decimal::ONE_HUNDRED),
    ))
}
