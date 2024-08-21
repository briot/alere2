mod networth_view;
pub mod tables;

use crate::networth_view::networth_view;
use alere_lib::{
    accounts::AccountNameKind, importers::Importer, kmymoney::KmyMoneyImporter,
    networth::Networth,
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
    let repo = block_on(kmy.import_file(
        Path::new("./Comptes.kmy"),
        |current, max| {
            progress.set_length(max);
            progress.set_position(current);
        },
    ))?;

    let now = Local::now();
    let networth = Networth::new(
        &repo,
        &[now - chrono::Months::new(1), now],
        alere_lib::networth::Settings {
            column_market: true,
            column_value: false,
            column_delta: false,
            column_market_delta: true,
            hide_zero: true,
            hide_all_same: false,
            tree: true,
            account_names: AccountNameKind::Short,
            subtotals: true,
            commodity: repo.find_commodity("Euro"),
        },
    );

    let output = networth_view(&repo, networth);
    progress.finish_and_clear();
    println!("{}", output);

    Ok(())
}
