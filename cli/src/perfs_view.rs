use crate::{
    global_settings::GlobalSettings,
    tables::{Align, Column, ColumnFooter, Table, Truncate, Width},
};
use alere_lib::{
    accounts::AccountNameDepth, multi_values::MultiValue, perf::Performance,
    repositories::Repository,
};
use anyhow::Result;
use console::Term;
use rust_decimal::Decimal;

fn returns(val: &Option<Decimal>) -> String {
    val.map(|p| format!("{:.2}%", ((p - Decimal::ONE) * Decimal::ONE_HUNDRED)))
        .unwrap_or("n/a".to_string())
}

pub fn perfs_view(
    repo: &Repository,
    globals: &GlobalSettings,
) -> Result<String> {
    let mut perfs = Performance::load(
        repo,
        alere_lib::perf::Settings {
            commodity: globals.commodity.clone(),
            //            intervals: vec![
            //                Intv::Yearly {
            //                    begin: Instant::StartYear(2022),
            //                    end: Instant::EndYear(2024),
            //                },
            //                Intv::LastNYears(1),
            //                Intv::YearToDate,
            //            ],
        },
        globals.reftime,
    )?;

    let mv = |val: &Option<MultiValue>| {
        if let Some(a) = val {
            a.display(&globals.format)
        } else {
            "".to_string()
        }
    };
    let account_image = |row: &Performance, _idx: &usize| {
        row.account.name(AccountNameDepth::unlimited())
    };
    let equity_image =
        |row: &Performance, _idx: &usize| row.equity.display(&globals.format);
    let shares_image =
        |row: &Performance, _idx: &usize| row.shares.display(&globals.format);
    let invested_image =
        |row: &Performance, _idx: &usize| row.invested.display(&globals.format);
    let realized_image =
        |row: &Performance, _idx: &usize| row.realized.display(&globals.format);
    let roi_image = |row: &Performance, _idx: &usize| returns(&row.roi);
    let pnl_image =
        |row: &Performance, _idx: &usize| row.pnl.display(&globals.format);
    let weighted_avg_image =
        |row: &Performance, _idx: &usize| mv(&row.weighted_average);
    let avg_cost_image =
        |row: &Performance, _idx: &usize| mv(&row.average_cost);
    let price_image = |row: &Performance, _idx: &usize| mv(&row.price);

    let columns = vec![
        Column::new(0, &account_image)
            .show_indent()
            .with_title("Account")
            .with_footer(ColumnFooter::Hide)
            .with_width(Width::ExpandWithMin(15))
            .with_truncate(Truncate::Left),
        Column::new(0, &equity_image)
            .with_title("Equity")
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &invested_image)
            .with_title("Invested")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &realized_image)
            .with_title("Realized")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &roi_image)
            .with_title("Return")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &pnl_image)
            .with_title("P&L")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &weighted_avg_image)
            .with_title("WAvg")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &avg_cost_image)
            .with_title("Avg Cost")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &price_image)
            .with_title("Price")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
        Column::new(0, &shares_image)
            .with_title("Shares")
            .with_footer(ColumnFooter::Hide)
            .with_align(Align::Right)
            .with_truncate(Truncate::Left),
    ];

    let mut table = Table::new(columns, &globals.table).with_col_headers();
    perfs.sort_by_key(|p| account_image(p, &0));
    for row in &perfs {
        if !row.invested.is_zero() {
            table.add_row(row, 0);
        }
    }

    Ok(table.to_string(Term::stdout().size().1 as usize))
}
