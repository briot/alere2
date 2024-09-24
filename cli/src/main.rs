mod networth_view;
mod stats_view;
pub mod tables;

use crate::{networth_view::networth_view, stats_view::stats_view};
use alere_lib::{
    account_categories::AccountCategory,
    accounts::AccountNameDepth,
    formatters::{Formatter, SymbolQuote},
    hledger::Hledger,
    importers::{Exporter, Importer},
    kmymoney::KmyMoneyImporter,
    networth::{GroupBy, Networth},
    stats::Stats,
    times::{Instant, Interval},
};
use anyhow::Result;
use chrono::Local;
use futures::executor::block_on;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

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
    let mut repo = block_on(kmy.import_file(
        Path::new("./Comptes.kmy"),
        |current, max| {
            progress.set_length(max);
            progress.set_position(current);
        },
    ))?;
    progress.finish_and_clear();

    let now = Local::now();

    let mut hledger = Hledger {
        export_reconciliation: false,
        assertions: alere_lib::hledger::AssertionMode::AtTime(vec![
            Instant::Now,
        ]),
    };
    repo.format = Formatter {
        quote_symbol: SymbolQuote::QuoteSpecial,
        zero: "0",
        //  separators: Separators::None,
        ..Formatter::default()
    };
    hledger.export_file(&repo, Path::new("./hledger.journal"))?;
    println!("Run
hledger -f hledger.journal bal --value=end,€  --end=today --tree Asset Liability");

    repo.format = Formatter::default();

    let output = networth_view(
        &repo,
        Networth::new(
            &repo,
            alere_lib::networth::Settings {
                hide_zero: true,
                hide_all_same: false,
                group_by: GroupBy::ParentAccount,
                subtotals: true,
                commodity: repo.commodities.find("Euro"),
                elide_boring_accounts: true,
                intervals: vec![
                    Interval::UpTo(Instant::YearsAgo(1)),
                    Interval::UpTo(Instant::MonthsAgo(1)),
                    Interval::UpTo(Instant::Now),
                ],
            },
            now,
            |(_acc_id, acc)| {
                repo.account_kinds.get(acc.kind).unwrap().is_networth
            },
        )?,
        crate::networth_view::Settings {
            column_market: true,
            column_value: false,
            column_delta: false,
            column_delta_to_last: false,
            column_price: false,
            column_market_delta: false,
            column_market_delta_to_last: false,
            column_percent: false,
            account_names: AccountNameDepth(1),
            table: crate::tables::Settings {
                colsep: "│".to_string(),
                indent_size: 1,
            },
        },
    );
    println!("{}", output.unwrap());

    let income_expenses = networth_view(
        &repo,
        Networth::new(
            &repo,
            alere_lib::networth::Settings {
                hide_zero: true,
                hide_all_same: false,
                group_by: GroupBy::ParentAccount,
                subtotals: true,
                commodity: repo.commodities.find("Euro"),
                elide_boring_accounts: true,
                intervals: vec![
                    Interval::Yearly {
                        begin: Instant::YearsAgo(2),
                        end: Instant::YearsAgo(1),
                    },
                    Interval::LastNYears(1),
                ],
            },
            now,
            |(_acc_id, acc)| {
                matches!(
                    repo.account_kinds.get(acc.kind).unwrap().category,
                    AccountCategory::EXPENSE
                    | AccountCategory::INCOME
                )
            },
        )?,
        crate::networth_view::Settings {
            column_market: true,
            column_value: false,
            column_delta: false,
            column_delta_to_last: false,
            column_price: false,
            column_market_delta: false,
            column_market_delta_to_last: false,
            column_percent: false,
            account_names: AccountNameDepth(1),
            table: crate::tables::Settings {
                colsep: "│".to_string(),
                indent_size: 1,
            },
        },
    );
    println!("{}", income_expenses.unwrap());

    let output = stats_view(
        &repo,
        Stats::new(
            &repo,
            alere_lib::stats::Settings {
                commodity: repo.commodities.find("Euro"),
                over: Interval::LastNYears(1),
            },
            now,
        )?,
        crate::stats_view::Settings {},
    );
    println!("{}", output);

    Ok(())
}
