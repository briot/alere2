mod args;
mod networth_view;
mod stats_view;
pub mod tables;

use crate::{
    args::build_cli, networth_view::networth_view, stats_view::stats_view,
};
use alere_lib::{
    account_categories::AccountCategory,
    accounts::AccountNameDepth,
    commodities::CommodityId,
    formatters::{Formatter, SymbolQuote},
    hledger::Hledger,
    importers::{Exporter, Importer},
    kmymoney::KmyMoneyImporter,
    networth::{GroupBy, Networth},
    repositories::Repository,
    stats::Stats,
    times::{Instant, Intv},
};
use anyhow::Result;
use chrono::Local;
use futures::executor::block_on;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Export all transaction to hledger format
fn export_hledger(repo: &mut Repository, output: &Path) -> Result<()> {
    repo.format = Formatter {
        quote_symbol: SymbolQuote::QuoteSpecial,
        zero: "0",
        //  separators: Separators::None,
        ..Formatter::default()
    };

    let mut hledger = Hledger {
        export_reconciliation: false,
        assertions: alere_lib::hledger::AssertionMode::AtTime(vec![
            Instant::Now,
        ]),
    };
    hledger.export_file(repo, output)?;
    println!(
        "Run
hledger -f {} bal --value=end,€ --end=today --tree Asset Liability",
        output.display()
    );

    Ok(())
}

/// Display stats
fn stats(repo: &Repository, commodity: Option<CommodityId>) -> Result<()> {
    let now = Local::now();
    let output = stats_view(
        repo,
        Stats::new(
            repo,
            alere_lib::stats::Settings {
                commodity,
                over: Intv::LastNYears(1),
            },
            now,
        )?,
        crate::stats_view::Settings {},
    );
    println!("{}", output);
    Ok(())
}

/// Show networth
fn networth(
    repo: &mut Repository,
    commodity: Option<CommodityId>,
) -> Result<()> {
    let now = Local::now();
    repo.format = Formatter::default();
    let output = networth_view(
        repo,
        Networth::new(
            repo,
            alere_lib::networth::Settings {
                hide_zero: true,
                hide_all_same: false,
                group_by: GroupBy::ParentAccount,
                subtotals: true,
                commodity,
                elide_boring_accounts: true,
                intervals: vec![
                    Intv::UpTo(Instant::YearsAgo(1)),
                    Intv::UpTo(Instant::MonthsAgo(1)),
                    Intv::UpTo(Instant::Now),
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
    Ok(())
}

/// Show income-expenses
fn cashflow(
    repo: &mut Repository,
    commodity: Option<CommodityId>,
) -> Result<()> {
    let now = Local::now();
    repo.format = Formatter::default();
    let income_expenses = networth_view(
        repo,
        Networth::new(
            repo,
            alere_lib::networth::Settings {
                hide_zero: true,
                hide_all_same: false,
                group_by: GroupBy::ParentAccount,
                subtotals: true,
                commodity,
                elide_boring_accounts: true,
                intervals: vec![
                    Intv::Yearly {
                        begin: Instant::YearsAgo(2),
                        end: Instant::YearsAgo(1),
                    },
                    Intv::LastNYears(1),
                ],
            },
            now,
            |(_acc_id, acc)| {
                matches!(
                    repo.account_kinds.get(acc.kind).unwrap().category,
                    AccountCategory::EXPENSE | AccountCategory::INCOME
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
    Ok(())
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
    let mut repo = block_on(kmy.import_file(
        Path::new("./Comptes.kmy"),
        |current, max| {
            progress.set_length(max);
            progress.set_position(current);
        },
    ))?;
    progress.finish_and_clear();

    let args = build_cli().get_matches();
    let commodity = args
        .get_one::<String>("currency")
        .and_then(|m| repo.commodities.find(m));
    match args.subcommand() {
        Some(("completions", sub_matches)) => {
            if let Some(shell) =
                sub_matches.get_one::<clap_complete_command::Shell>("shell")
            {
                let mut command = build_cli();
                shell.generate(&mut command, &mut std::io::stdout());
            }
        }
        Some(("export", sub)) => match sub.subcommand() {
            Some(("hledger", sub)) => {
                export_hledger(
                    &mut repo,
                    Path::new(
                        sub.get_one::<String>("output").expect("required"),
                    ),
                )?;
            }
            _ => unreachable!(),
        },
        Some(("networth", _)) => {
            networth(&mut repo, commodity)?;
        }
        Some(("cashflow", _)) => {
            cashflow(&mut repo, commodity)?;
        }
        Some(("stats", _)) => {
            stats(&repo, commodity)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
