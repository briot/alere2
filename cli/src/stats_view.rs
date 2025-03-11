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
            commodity: globals.commodity,
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
        repo.display_multi_value(&stats.mkt_start_networth, &globals.format),
        repo.display_multi_value(&stats.mkt_end_networth, &globals.format),
        repo.display_multi_value(&abs_income, &globals.format),
        repo.display_multi_value(&abs_passive_income, &globals.format),
        repo.display_multi_value(&neg_expense, &globals.format),
        repo.display_multi_value(&mkt_cashflow, &globals.format),
        repo.display_multi_value(&stats.mkt_unrealized, &globals.format),
        repo.display_multi_value(
            &(&mkt_cashflow + &stats.mkt_unrealized),
            &globals.format
        ),
        repo.display_multi_value(
            &(&stats.mkt_end_networth - &stats.mkt_start_networth),
            &globals.format,
        ),
        (&mkt_cashflow / &abs_income).map(|p| p * Decimal::ONE_HUNDRED),
        (&(&abs_passive_income + &stats.mkt_unrealized) / &stats.mkt_expense)
            .map(|p| p * Decimal::ONE_HUNDRED),
        (&(&abs_passive_income + &stats.mkt_unrealized) / &abs_income)
            .map(|p| p * Decimal::ONE_HUNDRED),
    ))
}
