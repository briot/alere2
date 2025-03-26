use crate::global_settings::GlobalSettings;
use alere_lib::{repositories::Repository, metrics::Metrics, times::Intv};
use anyhow::Result;
use rust_decimal::Decimal;

fn percent(val: Option<Decimal>) -> String {
    val.map(|p| format!("{:.1}%", (p * Decimal::ONE_HUNDRED)))
        .unwrap_or("n/a".to_string())
}

pub fn metrics_view(
    repo: &Repository,
    globals: &GlobalSettings,
) -> Result<String> {
    let m = Metrics::new(
        repo,
        alere_lib::metrics::Settings {
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
        m.start_networth.display(&globals.format),
        m.end_networth.display(&globals.format),
        m.pnl.display(&globals.format),
        (-&m.income).display(&globals.format),
        (-&m.passive_income).display(&globals.format),
        (-&m.expense).display(&globals.format),
        (-&m.cashflow).display(&globals.format),
        m.unrealized.display(&globals.format),
        percent(m.saving_rate),
        percent(m.financial_independence),
        percent(m.passive_income_ratio),
    ))
}
