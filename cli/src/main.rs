mod networth_view;
pub mod tables;

use crate::networth_view::networth_view;
use alere_lib::{
    accounts::AccountNameKind,
    importers::Importer,
    kmymoney::KmyMoneyImporter,
    networth::Networth,
    times::{get_timestamps, Timestamp},
};
use anyhow::Result;
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

    let output = networth_view(
        &repo,
        Networth::new(
            &repo,
            &get_timestamps(&[Timestamp::YearsAgo(1), Timestamp::MonthsAgo(1), Timestamp::Now])
                .collect::<Vec<_>>(),
            alere_lib::networth::Settings {
                hide_zero: true,
                hide_all_same: false,
                tree: true,
                subtotals: true,
                commodity: repo.find_commodity("Euro"),
            },
        ),
        crate::networth_view::Settings {
            column_market: true,
            column_value: false,
            column_delta: false,
            column_delta_to_last: false,
            column_price: false,
            column_market_delta: false,
            column_market_delta_to_last: true,
            account_names: AccountNameKind::Short,
        },
    );
    progress.finish_and_clear();
    println!("{}", output);

    Ok(())
}
