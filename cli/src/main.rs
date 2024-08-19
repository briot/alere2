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
use std::path::Path;

#[derive(Default)]
struct BalanceViewSettings {
    column_value: bool,
    column_market: bool,
}

fn balance_view(
    repo: &Repository,
    as_of: &[DateTime<Local>],
    settings: BalanceViewSettings,
) -> String {
    #[derive(Default, Clone)]
    struct Row {
        value: MultiValue,
        market_value: MultiValue,
        account_name: String,
    }
    let mv_image = |row: &Row| repo.display_multi_value(&row.value);
    let market_image = |row: &Row| repo.display_multi_value(&row.market_value);

    let mut market = repo.market_prices(repo.find_commodity("Euro"));
    let mut lines = Vec::new();
    let mut total = Row::default();
    for (account, value) in repo.balance(as_of[0]) {
        if !value.is_zero() {
            let market_value = market.convert_multi_value(&value, &as_of[0]);
            total.value += &value;
            total.market_value += &market_value;

            lines.push(Row {
                value,
                market_value,
                account_name: repo
                    .get_account_name(account, AccountNameKind::Full),
            });
        }
    }
    lines.sort_by(|l1, l2| l1.account_name.cmp(&l2.account_name));

    let mut columns = Vec::new();
    if settings.column_value {
        columns.push(
            Column::new("Value", &mv_image)
                .with_align(Align::Right)
                .with_truncate(Truncate::Left)
                .with_footer(ColumnFooter::Hide),
        );
    }
    if settings.column_market {
        columns.push(
            Column::new("Market", &market_image)
                .with_align(Align::Right)
                .with_truncate(Truncate::Left),
        );
    }
    columns.push(
        Column::new("Account", &|row: &Row| row.account_name.clone())
            .with_width(Width::Expand)
            .with_truncate(Truncate::Left)
            .with_footer(ColumnFooter::Hide),
    );

    let mut table = Table::<Row>::new(columns)
        .with_title(&format!("Balance as of {}", as_of[0]));
    table.add_col_headers();
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
        },
    );
    progress.finish_and_clear();
    println!("{}", output);

    Ok(())
}
