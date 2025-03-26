use crate::global_settings::GlobalSettings;
use alere_lib::{repositories::Repository, stats::Stats, times::Intv};
use anyhow::Result;
use rust_decimal::Decimal;

fn percent(val: Option<Decimal>) -> String {
    val.map(|p| format!("{:.1}%", (p * Decimal::ONE_HUNDRED)))
        .unwrap_or("n/a".to_string())
}

pub fn stats_view(
    repo: &Repository,
    globals: &GlobalSettings,
) -> Result<String> {
    let stats = Stats::new(
        repo,
        alere_lib::stats::Settings {
            commodity: globals.commodity.clone(),
            over: Intv::LastNYears(1),
            // over: Intv::YearAgo(1),
        },
        globals.reftime,
    )?;

    Ok(format!(
        "
Networth:        {} to {}
P&L:             {}
Income:          {} = passive {} + salaries + ...
Expenses:        {}
Cashflow:        {}
Unrealized:      + {}
Savings Rate:    {}
Financial Indep: {}
Passive Income:  {}",
        stats.start_networth.display(&globals.format),
        stats.end_networth.display(&globals.format),
        stats.pnl.display(&globals.format),
        (-&stats.income).display(&globals.format),
        (-&stats.passive_income).display(&globals.format),
        (-&stats.expense).display(&globals.format),
        (-&stats.cashflow).display(&globals.format),
        stats.unrealized.display(&globals.format),
        percent(stats.saving_rate),
        percent(stats.financial_independence),
        percent(stats.passive_income_ratio),
    ))
}
