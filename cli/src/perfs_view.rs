use crate::global_settings::GlobalSettings;
use alere_lib::{
    accounts::AccountNameDepth, multi_values::MultiValue, perf::Performance,
    repositories::Repository,
};
use anyhow::Result;
use rust_decimal::Decimal;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Modify, Style, object::Columns},
};

fn returns(val: &Option<Decimal>) -> String {
    val.map(|p| format!("{:.2}%", ((p - Decimal::ONE) * Decimal::ONE_HUNDRED)))
        .unwrap_or("n/a".to_string())
}

#[derive(Tabled)]
struct PerfRow {
    #[tabled(rename = "Account")]
    account: String,
    #[tabled(rename = "Equity")]
    equity: String,
    #[tabled(rename = "Invested")]
    invested: String,
    #[tabled(rename = "Realized")]
    realized: String,
    #[tabled(rename = "Return")]
    roi: String,
    #[tabled(rename = "P&L")]
    pnl: String,
    #[tabled(rename = "WAvg")]
    weighted_avg: String,
    #[tabled(rename = "Avg Cost")]
    avg_cost: String,
    #[tabled(rename = "Price")]
    price: String,
    #[tabled(rename = "Shares")]
    shares: String,
}

impl PerfRow {
    fn from_perf(
        perf: &Performance,
        format: &alere_lib::formatters::Formatter,
    ) -> Self {
        let mv = |val: &Option<MultiValue>| {
            val.as_ref().map(|a| a.display(format)).unwrap_or_default()
        };
        PerfRow {
            account: perf.account.name(AccountNameDepth::unlimited()),
            equity: perf.equity.display(format),
            invested: perf.invested.display(format),
            realized: perf.realized.display(format),
            roi: returns(&perf.roi),
            pnl: perf.pnl.display(format),
            weighted_avg: mv(&perf.weighted_average),
            avg_cost: mv(&perf.average_cost),
            price: mv(&perf.price),
            shares: perf.shares.display(format),
        }
    }
}

pub fn perfs_view(
    repo: &Repository,
    globals: &GlobalSettings,
) -> Result<String> {
    let mut perfs = Performance::load(
        repo,
        alere_lib::perf::Settings {
            commodity: globals.commodity.clone(),
        },
        globals.reftime,
    )?;

    perfs.sort_by_key(|p| p.account.name(AccountNameDepth::unlimited()));

    let rows: Vec<PerfRow> = perfs
        .iter()
        .filter(|p| !p.invested.is_zero())
        .map(|p| PerfRow::from_perf(p, &globals.format))
        .collect();

    let mut table = Table::new(rows);
    table
        .with(Style::modern())
        .with(Modify::new(Columns::new(1..)).with(Alignment::right()));

    Ok(table.to_string())
}
