use crate::{
    global_settings::GlobalSettings,
    tables::{Align, Column, Table, Truncate, Width},
};
use alere_lib::{
    metrics::Metrics,
    repositories::Repository,
    times::{Instant, Intv},
};
use anyhow::Result;
use console::Term;
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
        format!("{}y {}m", years, months)
    })
    .unwrap_or("n/a".to_string())
}

struct TableRow {
    name: String,
    values: Vec<String>,
}

impl TableRow {
    fn new<F>(name: &str, metrics: &[Metrics], get: F) -> Self
    where
        F: FnMut(&Metrics) -> String,
    {
        TableRow {
            name: name.to_string(),
            values: metrics.iter().map(get).collect(),
        }
    }

    fn name(&self, _idx: &usize) -> String {
        self.name.to_string()
    }

    fn image(&self, idx: &usize) -> String {
        self.values[*idx].clone()
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
                    end: Instant::EndYear(2024),
                },
                Intv::LastNYears(1),
                Intv::YearToDate,
            ],
        },
        globals.reftime,
    )?;

    let mut columns = vec![Column::new(0, &TableRow::name)
        .with_title("Metric")
        .with_width(Width::ExpandWithMin(8))
        .with_truncate(Truncate::Right)
        .with_align(Align::Left)];
    for (idx, s) in m.iter().enumerate() {
        columns.push(
            Column::new(idx, &TableRow::image)
                .with_title(&s.interval.descr)
                .with_align(Align::Right)
                .with_truncate(Truncate::Left),
        );
    }

    let mut table = Table::new(columns, &globals.table).with_col_headers();
    table.add_rows(
        &[
            TableRow::new("networth", &m, |s| {
                s.end_networth.display(&globals.format)
            }),
            TableRow::new("Income", &m, |s| {
                (-&s.income).display(&globals.format)
            }),
            TableRow::new("  work", &m, |s| {
                (-&s.work_income).display(&globals.format)
            }),
            TableRow::new("  passive", &m, |s| {
                (-&s.passive_income).display(&globals.format)
            }),
            TableRow::new("Expense", &m, |s| {
                (-&s.expense).display(&globals.format)
            }),
            TableRow::new("  Income tax", &m, |s| {
                (-&s.income_tax).display(&globals.format)
            }),
            TableRow::new("  Misc tax", &m, |s| {
                (-&s.misc_tax).display(&globals.format)
            }),
            TableRow::new("Cashflow", &m, |s| {
                (-&s.cashflow).display(&globals.format)
            }),
            TableRow::new("Unrealized", &m, |s| {
                s.unrealized.display(&globals.format)
            }),
            TableRow::new("  Liquid", &m, |s| {
                s.unrealized_liquid.display(&globals.format)
            }),
            TableRow::new("  Illiquid", &m, |s| {
                s.unrealized_illiquid.display(&globals.format)
            }),
            TableRow::new("P&L", &m, |s| s.pnl.display(&globals.format)),
            TableRow::new("  Liquid", &m, |s| {
                s.pnl_liquid.display(&globals.format)
            }),
            TableRow::new("  Illiquid", &m, |s| {
                s.pnl_illiquid.display(&globals.format)
            }),
            TableRow::new("Saving Rate", &m, |s| percent(&s.saving_rate)),
            TableRow::new("Financial Independence", &m, |s| {
                percent(&s.financial_independence)
            }),
            TableRow::new("Passive Income Ratio", &m, |s| {
                percent(&s.passive_income_ratio)
            }),
            TableRow::new("Return on Investment", &m, |s| percent(&s.roi)),
            TableRow::new("  Liquid", &m, |s| percent(&s.roi_liquid)),
            TableRow::new("Emergency Fund", &m, |s| {
                duration(&s.emergency_fund)
            }),
            TableRow::new("Wealth", &m, |s| duration(&s.wealth)),
            TableRow::new("Income Tax Rate", &m, |s| {
                percent(&s.income_tax_rate)
            }),
        ],
        0,
    );

    Ok(table.to_string(Term::stdout().size().1 as usize))
}
