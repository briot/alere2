use crate::global_settings::GlobalSettings;
use alere_lib::{
    metrics::Metrics,
    repositories::Repository,
    times::Intv,
};
use anyhow::Result;
use rust_decimal::Decimal;
use tabled::{
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

struct MetricRow {
    name: String,
    values: Vec<String>,
}

impl MetricRow {
    fn new<F>(name: &str, metrics: &[Metrics], mut get: F) -> Self
    where
        F: FnMut(&Metrics) -> String,
    {
        MetricRow {
            name: name.to_string(),
            values: metrics.iter().map(|m| get(m)).collect(),
        }
    }
}

pub fn metrics_view(
    repo: &Repository,
    globals: &GlobalSettings,
    periods: Vec<Intv>,
) -> Result<String> {
    let m = Metrics::load(
        repo,
        alere_lib::metrics::Settings {
            commodity: globals.commodity.clone(),
            intervals: periods,
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

    // Build table dynamically
    let mut builder = tabled::builder::Builder::default();
    
    // Header row
    let mut header = vec!["Metric".to_string()];
    header.extend(m.iter().map(|metric| metric.interval.descr.clone()));
    builder.push_record(header);
    
    // Data rows
    for row in rows {
        let mut record = vec![row.name];
        record.extend(row.values);
        builder.push_record(record);
    }
    
    let mut table = builder.build();
    globals.style.apply(&mut table);
    table.with(Modify::new(Columns::new(1..)).with(Alignment::right()));

    crate::global_settings::limit_table_width(&mut table, 0);

    Ok(table.to_string())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_row_dynamic_columns() {
        let metrics = vec![];
        let row = MetricRow::new("Test", &metrics, |_| "value".to_string());
        assert_eq!(row.name, "Test");
        assert_eq!(row.values.len(), 0);

        // Test with multiple columns
        let row = MetricRow::new("Test", &metrics, |_| "value".to_string());
        assert_eq!(row.values.len(), 0);
    }
}
