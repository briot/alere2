mod networth_view;
mod stats_view;
pub mod tables;

use crate::{networth_view::networth_view, stats_view::stats_view};
use alere_lib::{
    accounts::AccountNameKind,
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

    let mut hledger = Hledger {
        export_reconciliation: false,
    };
    repo.format = Formatter {
        quote_symbol: SymbolQuote::QuoteSpecial,
        zero: "0",
        //  separators: Separators::None,
        ..Formatter::default()
    };
    hledger.export_file(&repo, Path::new("./hledger.journal"))?;

    let now = Local::now();

    repo.format = Formatter::default();

    let output = networth_view(
        &repo,
        Networth::new(
            &repo,
            &[Instant::YearsAgo(1), Instant::Now]
                .iter()
                .map(|ts| ts.to_time(now))
                .collect::<Vec<_>>(),
            alere_lib::networth::Settings {
                hide_zero: true,
                hide_all_same: false,
                group_by: GroupBy::ParentAccount,
                subtotals: true,
                commodity: repo.commodities.find("Euro"),
            },
        ),
        crate::networth_view::Settings {
            column_market: true,
            column_value: false,
            column_delta: false,
            column_delta_to_last: false,
            column_price: false,
            column_market_delta: false,
            column_market_delta_to_last: false,
            column_percent: false,
            account_names: AccountNameKind::Short,
            table: crate::tables::Settings {
                colsep: "│".to_string(),
                indent_size: 1,
            },
        },
    );
    progress.finish_and_clear();
    println!("{}", output.unwrap());

    let output = stats_view(
        &repo,
        Stats::new(
            &repo,
            Interval::Years(1),
            alere_lib::stats::Settings {
                commodity: repo.commodities.find("Euro"),
            },
            now,
        ),
        crate::stats_view::Settings {},
    );
    progress.finish_and_clear();
    println!("{}", output);

    Ok(())
}
