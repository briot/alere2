pub mod tables;
pub mod networth;

use alere_lib::importers::Importer;
use alere_lib::kmymoney::KmyMoneyImporter;
use anyhow::Result;
use chrono::Local;
use crate::networth::networth_view;
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
    let repo = block_on(kmy.import_file(
        Path::new("./Comptes.kmy"),
        |current, max| {
            progress.set_length(max);
            progress.set_position(current);
        },
    ))?;

    let now = Local::now();
    let output = networth_view(
        &repo,
        &[now - chrono::Months::new(1), now],
        crate::networth::Settings {
            column_market: true,
            column_value: false,
            column_delta: false,
            column_market_delta: true,
            hide_zero: true,
            hide_all_same: false,
            tree: true,
            subtotals: true,
            commodity: repo.find_commodity("Euro"),
        },
    );
    progress.finish_and_clear();
    println!("{}", output);

    Ok(())
}
