use crate::global_settings::GlobalSettings;
use alere_lib::{
    metrics::Metrics,
    repositories::Repository,
    times::{Instant, Intv},
};
use anyhow::Result;
use rust_decimal::Decimal;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Modify, object::Columns},
};

fn percent(val: &Option<Decimal>) -> String {
    val.map(|p| format!("{:.2}%", (p * Decimal::ONE_HUNDRED)))
        .unwrap_or("n/a".to_string())
}

fn duration(val: &Option<Decimal>) -> String {
    val.map(|p| {
        let days_in_year = Decimal::from(365_i16);
        let days_in_month = days_in_year / Decimal::from(12_i8);
        let years = (p / days_in_year).floor();
        let months = ((p - years * days_in_year) / days_in_month).floor();
        format!("{}y {}m", years, months)
    })
    .unwrap_or("n/a".to_string())
}

#[derive(Tabled)]
struct MetricRow {
    #[tabled(rename = "Metric")]
    name: String,
    #[tabled(rename = "")]
    col1: String,
    #[tabled(rename = "")]
    col2: String,
    #[tabled(rename = "")]
    col3: String,
    #[tabled(rename = "")]
    col4: String,
}

impl MetricRow {
    fn new<F>(name: &str, metrics: &[Metrics], get: F) -> Self
    where
        F: FnMut(&Metrics) -> String,
    {
        let values: Vec<String> = metrics.iter().map(get).collect();
        MetricRow {
            name: name.to_string(),
            col1: values.first().cloned().unwrap_or_default(),
            col2: values.get(1).cloned().unwrap_or_default(),
            col3: values.get(2).cloned().unwrap_or_default(),
            col4: values.get(3).cloned().unwrap_or_default(),
        }
    }
}

pub fn metrics_view(
    repo: &Repository,
    globals: &GlobalSettings,
) -> Result<String> {
    let m = Metrics::load(
        repo,
        alere_lib::metrics::Settings {
            commodity: globals.commodity.clone(),
            intervals: vec![
                Intv::Yearly {
                    begin: Instant::StartYear(2022),
                    end: Instant::EndYear(2025),
                },
                Intv::LastNMonths(1),
                Intv::LastNYears(1),
                Intv::YearToDate,
            ],
        },
        globals.reftime,
    )?;

    let rows = vec![
        MetricRow::new("networth at end", &m, |s| {
            s.end_networth.display(&globals.format)
        }),
        MetricRow::new("Income", &m, |s| (-&s.income).display(&globals.format)),
        MetricRow::new("  work", &m, |s| {
            (-&s.work_income).display(&globals.format)
        }),
        MetricRow::new("  passive", &m, |s| {
            (-&s.passive_income).display(&globals.format)
        }),
        MetricRow::new("Expense", &m, |s| {
            (-&s.expense).display(&globals.format)
        }),
        MetricRow::new("  Income tax", &m, |s| {
            (-&s.income_tax).display(&globals.format)
        }),
        MetricRow::new("  Misc tax", &m, |s| {
            (-&s.misc_tax).display(&globals.format)
        }),
        MetricRow::new("Cashflow", &m, |s| {
            (-&s.cashflow).display(&globals.format)
        }),
        MetricRow::new("Unrealized", &m, |s| {
            s.unrealized.display(&globals.format)
        }),
        MetricRow::new("  Liquid", &m, |s| {
            s.unrealized_liquid.display(&globals.format)
        }),
        MetricRow::new("  Illiquid", &m, |s| {
            s.unrealized_illiquid.display(&globals.format)
        }),
        MetricRow::new("P&L", &m, |s| s.pnl.display(&globals.format)),
        MetricRow::new("  Liquid", &m, |s| {
            s.pnl_liquid.display(&globals.format)
        }),
        MetricRow::new("  Illiquid", &m, |s| {
            s.pnl_illiquid.display(&globals.format)
        }),
        MetricRow::new("Saving Rate", &m, |s| percent(&s.saving_rate)),
        MetricRow::new("Financial Independence", &m, |s| {
            percent(&s.financial_independence)
        }),
        MetricRow::new("Passive Income Ratio", &m, |s| {
            percent(&s.passive_income_ratio)
        }),
        MetricRow::new("Return on Investment", &m, |s| percent(&s.roi)),
        MetricRow::new("  Liquid", &m, |s| percent(&s.roi_liquid)),
        MetricRow::new("Emergency Fund", &m, |s| duration(&s.emergency_fund)),
        MetricRow::new("Wealth", &m, |s| duration(&s.wealth)),
        MetricRow::new("Income Tax Rate", &m, |s| percent(&s.income_tax_rate)),
    ];

    let mut table = Table::new(rows);
    globals.style.apply(&mut table);
    table.with(Modify::new(Columns::new(1..)).with(Alignment::right()));

    // Set column headers
    if let Some((first, _)) = m.split_first() {
        table.modify(
            tabled::settings::object::Rows::first(),
            tabled::settings::Format::content(|_| first.interval.descr.clone()),
        );
    }

    Ok(table.to_string())
}
