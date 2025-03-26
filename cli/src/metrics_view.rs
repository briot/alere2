use crate::global_settings::GlobalSettings;
use alere_lib::{metrics::Metrics, repositories::Repository, times::Intv};
use anyhow::Result;
use rust_decimal::Decimal;

fn percent(val: &Option<Decimal>) -> String {
    val.map(|p| format!("{:.2}%", (p * Decimal::ONE_HUNDRED)))
        .unwrap_or("n/a".to_string())
}

fn duration(val: &Option<Decimal>) -> String {
    val.map(|p| {
        let days_in_year = Decimal::from(365_i16);
        let days_in_month = days_in_year / Decimal::from(12_i8); // approximate
        let years = (p / days_in_year).floor();
        let months = ((p - years * days_in_year) / days_in_month).floor();
        format!("{} years {} months", years, months)
    })
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
            //over: Intv::LastNYears(1),
            over: Intv::YearAgo(1),
        },
        globals.reftime,
    )?;

    Ok(format!(
        "
Networth:        {} to {}
   = liquid      {} to {}
   + illiquid    {} to {}
P&L:             {}
   = liquid      {}
   + illiquid    {}
Income:          {} = passive {} + work {} + ...
Expenses:        {}
Cashflow:        {}
Unrealized:      + {}
   = liquid      {}
   + illiquid    {}
Savings Rate:    {}
Financial Indep: {}
Passive Income:  {}
ROI:             {}
ROI for liquid:  {}
Emergency fund:  {}
Wealth:          {}
Actual income tax rate: {}
Taxes=           income {} + misc {}
",
        m.start_networth.display(&globals.format),
        m.end_networth.display(&globals.format),
        m.start_networth_liquid.display(&globals.format),
        m.end_networth_liquid.display(&globals.format),
        m.start_networth_illiquid.display(&globals.format),
        m.end_networth_illiquid.display(&globals.format),
        m.pnl.display(&globals.format),
        m.pnl_liquid.display(&globals.format),
        m.pnl_illiquid.display(&globals.format),
        (-&m.income).display(&globals.format),
        (-&m.passive_income).display(&globals.format),
        (-&m.work_income).display(&globals.format),
        (-&m.expense).display(&globals.format),
        (-&m.cashflow).display(&globals.format),
        m.unrealized.display(&globals.format),
        m.unrealized_liquid.display(&globals.format),
        m.unrealized_illiquid.display(&globals.format),
        percent(&m.saving_rate),
        percent(&m.financial_independence),
        percent(&m.passive_income_ratio),
        percent(&m.roi),
        percent(&m.roi_liquid),
        duration(&m.emergency_fund),
        duration(&m.wealth),
        percent(&m.income_tax_rate),
        m.income_tax.display(&globals.format),
        m.misc_tax.display(&globals.format),
    ))
}
