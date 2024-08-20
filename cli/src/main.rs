pub mod tables;

use crate::tables::{Align, Column, ColumnFooter, Table, Truncate, Width};
use alere_lib::accounts::AccountNameKind;
use alere_lib::importers::Importer;
use alere_lib::kmymoney::KmyMoneyImporter;
use alere_lib::multi_values::MultiValue;
use alere_lib::repositories::Repository;
use anyhow::Result;
use chrono::{DateTime, Local};
use console::Term;
use futures::executor::block_on;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use std::path::Path;

/// Whether the vector contains all-equal elements
fn is_all_same<T: PartialEq>(arr: &[T]) -> bool {
    arr.windows(2).all(|w| w[0] == w[1])
}

#[derive(Default)]
struct BalanceViewSettings {
    column_value: bool,
    column_market: bool,
    column_delta: bool,
    column_market_delta: bool,

    // Do not show rows if the value is zero
    hide_zero: bool,

    // Do not show rows if the value or market_value hasn't changed between
    // all timestamps.  If there is a single timestamp, rows are always
    // displayed.
    hide_all_same: bool,
}

fn balance_view(
    repo: &Repository,
    as_of: &[DateTime<Local>],
    settings: BalanceViewSettings,
) -> String {
    #[derive(Clone)]
    struct Row {
        value: Vec<MultiValue>,
        market_value: Vec<MultiValue>,
        account_name: String,
    }

    let mut market = repo.market_prices(repo.find_commodity("Euro"));
    let mut lines = Vec::new();
    let mut total = Row {
        value: vec![MultiValue::default(); as_of.len()],
        market_value: vec![MultiValue::default(); as_of.len()],
        account_name: String::new(),
    };
    for (account, value) in repo.balance(as_of) {
        let mut row = Row {
            market_value: vec![MultiValue::default(); value.len()],
            value,
            account_name: repo.get_account_name(account, AccountNameKind::Full),
        };

        let mut has_non_zero = false;

        for (idx, v) in row.value.iter().enumerate() {
            if !v.is_zero() {
                has_non_zero = true;
                let market_value = market.convert_multi_value(v, &as_of[idx]);
                total.value[idx] += v;
                total.market_value[idx] += &market_value;
                row.market_value[idx] = market_value;
            }
        }

        if (!settings.hide_zero || has_non_zero)
            && (!settings.hide_all_same
                || !is_all_same(&row.value)
                || !is_all_same(&row.market_value))
        {
            lines.push(row);
        }
    }
    lines.sort_by(|l1, l2| l1.account_name.cmp(&l2.account_name));

    let mv_image =
        |row: &Row, idx: &usize| repo.display_multi_value(&row.value[*idx]);
    let market_image = |row: &Row, idx: &usize| {
        repo.display_multi_value(&row.market_value[*idx])
    };
    let delta_image = |row: &Row, idx: &usize| {
        repo.display_multi_value(&(&row.value[*idx + 1] - &row.value[*idx]))
    };
    let delta_market_image = |row: &Row, idx: &usize| {
        repo.display_multi_value(
            &(&row.market_value[*idx + 1] - &row.market_value[*idx]),
        )
    };

    let mut columns = Vec::new();
    for (pos, (idx, ts)) in as_of.iter().enumerate().with_position() {
        if settings.column_value {
            columns.push(
                Column::new(idx, &mv_image)
                    .with_title(&format!("Value {}", ts.date_naive()))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left)
                    .with_footer(ColumnFooter::Hide),
            );
        }
        if settings.column_market {
            columns.push(
                Column::new(idx, &market_image)
                    .with_title(&format!("Mkt {}", ts.date_naive()))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if let itertools::Position::Last | itertools::Position::Only = pos {
        } else {
            if settings.column_delta {
                columns.push(
                    Column::new(idx, &delta_image)
                        .with_title("Delta")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if settings.column_market_delta {
                columns.push(
                    Column::new(idx, &delta_market_image)
                        .with_title("Delta Mkt")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
        }
    }
    columns.push(
        Column::new(0, &|row: &Row, _| row.account_name.clone())
            .with_title("Account")
            .with_width(Width::Expand)
            .with_truncate(Truncate::Left)
            .with_footer(ColumnFooter::Hide),
    );

    let mut table = Table::new(columns).with_col_headers();
    //.with_colsep(" ");
    table.add_rows(lines);
    table.add_footer(&total);

    table.to_string(Term::stdout().size().1 as usize)
}

fn main() -> Result<()> {
    let progress = ProgressBar::new(1) //  we do not know the length
        .with_style(
            ProgressStyle::with_template(
                "[{pos:2}/{len:2}] {msg} {wide_bar} {elapsed_precise}",
            )
            .unwrap(),
        )
        .with_message("importing kmy");

    let mut kmy = KmyMoneyImporter::default();
    let repo = block_on(kmy.import_file(
        Path::new("./Comptes.kmy"),
        |current, max| {
            progress.set_length(max);
            progress.set_position(current);
        },
    ))?;

    let now = Local::now();
    let output = balance_view(
        &repo,
        &[now - chrono::Months::new(1), now],
        BalanceViewSettings {
            column_market: true,
            column_value: false,
            column_delta: false,
            column_market_delta: true,
            hide_zero: true,
            hide_all_same: false,
        },
    );
    progress.finish_and_clear();
    println!("{}", output);

    Ok(())
}
