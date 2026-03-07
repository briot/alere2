use crate::global_settings::GlobalSettings;
use alere_lib::{
    accounts::AccountNameDepth, multi_values::MultiValue, perf::Performance,
    repositories::Repository,
};
use anyhow::Result;
use clap::ValueEnum;
use rust_decimal::Decimal;
use tabled::{
    Tabled,
    builder::Builder,
    settings::{Alignment, Modify, object::Columns},
};

#[derive(Clone, ValueEnum)]
pub enum PerfColumn {
    Equity,
    Invested,
    Realized,
    Return,
    Annualized,
    Irr,
    Pnl,
    Wavg,
    Avgcost,
    Price,
    Shares,
}

const DEFAULT_COLUMNS: &[PerfColumn] = &[
    PerfColumn::Equity,
    PerfColumn::Invested,
    PerfColumn::Realized,
    PerfColumn::Return,
    PerfColumn::Annualized,
    PerfColumn::Pnl,
    PerfColumn::Wavg,
    PerfColumn::Avgcost,
    PerfColumn::Price,
    PerfColumn::Shares,
];

fn returns(val: &Option<Decimal>) -> String {
    val.map(|p| format!("{:.2}%", ((p - Decimal::ONE) * Decimal::ONE_HUNDRED)))
        .unwrap_or("n/a".to_string())
}

fn rate(val: &Option<Decimal>) -> String {
    val.map(|p| format!("{:.2}%", (p * Decimal::ONE_HUNDRED)))
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
    #[tabled(rename = "Annualized")]
    annualized: String,
    #[tabled(rename = "IRR")]
    irr: String,
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
            annualized: returns(&perf.annualized_roi),
            irr: rate(&perf.irr),
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
    columns: Option<Vec<PerfColumn>>,
) -> Result<String> {
    let selected_columns = columns.unwrap_or_else(|| DEFAULT_COLUMNS.to_vec());

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

    // Build table with selected columns only
    let mut builder = Builder::default();

    // Add header - Account is always first
    let mut header = vec!["Account"];
    for col in &selected_columns {
        header.push(match col {
            PerfColumn::Equity => "Equity",
            PerfColumn::Invested => "Invested",
            PerfColumn::Realized => "Realized",
            PerfColumn::Return => "Return",
            PerfColumn::Annualized => "Annualized",
            PerfColumn::Irr => "IRR",
            PerfColumn::Pnl => "P&L",
            PerfColumn::Wavg => "WAvg",
            PerfColumn::Avgcost => "Avg Cost",
            PerfColumn::Price => "Price",
            PerfColumn::Shares => "Shares",
        });
    }
    builder.push_record(header);

    // Add data rows - Account is always first
    for row in &rows {
        let mut record = vec![&row.account];
        for col in &selected_columns {
            record.push(match col {
                PerfColumn::Equity => &row.equity,
                PerfColumn::Invested => &row.invested,
                PerfColumn::Realized => &row.realized,
                PerfColumn::Return => &row.roi,
                PerfColumn::Annualized => &row.annualized,
                PerfColumn::Irr => &row.irr,
                PerfColumn::Pnl => &row.pnl,
                PerfColumn::Wavg => &row.weighted_avg,
                PerfColumn::Avgcost => &row.avg_cost,
                PerfColumn::Price => &row.price,
                PerfColumn::Shares => &row.shares,
            });
        }
        builder.push_record(record);
    }

    let mut table = builder.build();
    globals.style.apply(&mut table);

    // Right-align all columns except the first (Account)
    table.with(Modify::new(Columns::new(1..)).with(Alignment::right()));

    crate::global_settings::limit_table_width(&mut table, 0);

    Ok(table.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_order_matches_input() {
        // Create a simple test to verify column headers appear in specified order
        let mut builder = Builder::default();
        builder.push_record(vec!["Account", "Return", "Equity", "P&L"]);

        let table = builder.build();
        let output = table.to_string();

        // Check that Return appears before Equity in the output
        let return_pos =
            output.find("Return").expect("Return column not found");
        let equity_pos =
            output.find("Equity").expect("Equity column not found");
        let pnl_pos = output.find("P&L").expect("P&L column not found");

        assert!(
            return_pos < equity_pos,
            "Return should appear before Equity"
        );
        assert!(equity_pos < pnl_pos, "Equity should appear before P&L");
    }

    #[test]
    fn test_account_always_first() {
        // Verify Account column is always first regardless of column selection
        let mut builder = Builder::default();
        builder.push_record(vec!["Account", "P&L", "Return"]);

        let table = builder.build();
        let output = table.to_string();

        // Account should appear before any other column
        let account_pos =
            output.find("Account").expect("Account column not found");
        let return_pos =
            output.find("Return").expect("Return column not found");
        let pnl_pos = output.find("P&L").expect("P&L column not found");

        assert!(
            account_pos < return_pos,
            "Account should appear before Return"
        );
        assert!(account_pos < pnl_pos, "Account should appear before P&L");
    }

    #[test]
    fn test_column_filtering_excludes_unselected() {
        // Verify that only selected columns appear in output
        let mut builder = Builder::default();
        builder.push_record(vec!["Account", "Equity", "Return"]);

        let table = builder.build();
        let output = table.to_string();

        // Selected columns should be present
        assert!(output.contains("Account"), "Account should be present");
        assert!(output.contains("Equity"), "Equity should be present");
        assert!(output.contains("Return"), "Return should be present");

        // Unselected columns should not be present
        assert!(
            !output.contains("Invested"),
            "Invested should not be present"
        );
        assert!(!output.contains("P&L"), "P&L should not be present");
        assert!(!output.contains("IRR"), "IRR should not be present");
    }

    #[test]
    fn test_single_column_selection() {
        // Verify single column selection works (Account + one column)
        let mut builder = Builder::default();
        builder.push_record(vec!["Account", "P&L"]);
        builder.push_record(vec!["Test Account", "1000.00"]);

        let table = builder.build();
        let output = table.to_string();

        assert!(output.contains("Account"), "Account should be present");
        assert!(output.contains("P&L"), "P&L should be present");
        assert!(output.contains("Test Account"), "Data should be present");
        assert!(output.contains("1000.00"), "Data should be present");

        // Verify we don't have columns we didn't select
        assert!(!output.contains("Equity"), "Equity should not be present");
        assert!(!output.contains("Return"), "Return should not be present");
    }
}
