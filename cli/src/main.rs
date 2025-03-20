mod args;
mod global_settings;
mod networth_view;
mod stats_view;
pub mod tables;

use crate::{
    args::build_cli, global_settings::GlobalSettings,
    networth_view::networth_view, stats_view::stats_view,
};
use alere_lib::{
    account_categories::AccountCategory,
    formatters::{Formatter, SymbolQuote},
    hledger::Hledger,
    importers::{Exporter, Importer},
    kmymoney::KmyMoneyImporter,
    qif::QIF,
    repositories::Repository,
    times::Instant,
};
use anyhow::Result;
use clap::ArgMatches;
use futures::executor::block_on;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Export all transaction to hledger format
fn export_hledger(repo: &mut Repository, output: &Path) -> Result<()> {
    let format = Formatter {
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
    hledger.export_file(repo, output, &format)?;
    println!(
        "Run
hledger -f {} bal --value=end,€ --end=today --tree Asset Liability",
        output.display()
    );

    Ok(())
}

/// Export all transaction to QIF format
fn export_qif(repo: &mut Repository, output: &Path) -> Result<()> {
    let format = Formatter {
        quote_symbol: SymbolQuote::NeverQuote,
        zero: "0",
        ..Formatter::default()
    };
    let mut q = QIF {};
    q.export_file(repo, output, &format)?;
    Ok(())
}

/// Display stats
fn stats(repo: &Repository, globals: &GlobalSettings) -> Result<()> {
    let output = stats_view(
        repo,
        globals,
    )?;
    println!("{}", output);
    Ok(())
}

/// Show networth
fn networth(
    repo: &mut Repository,
    globals: &GlobalSettings,
    args: &ArgMatches,
) -> Result<()> {
    let output = networth_view(repo, args, globals, |(_acc_id, acc)| {
        repo.account_kinds.get(acc.kind).unwrap().is_networth
    })?;
    println!("{}", output);
    Ok(())
}

/// Show income-expenses
fn cashflow(
    repo: &mut Repository,
    globals: &GlobalSettings,
    args: &ArgMatches,
) -> Result<()> {
    let income_expenses =
        networth_view(repo, args, globals, |(_acc_id, acc)| {
            matches!(
                repo.account_kinds.get(acc.kind).unwrap().category,
                AccountCategory::EXPENSE | AccountCategory::INCOME
            )
        });
    println!("{}", income_expenses.unwrap());
    Ok(())
}

fn main() -> Result<()> {
    let args = build_cli().get_matches();
    let mut settings = GlobalSettings::new(&args);

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

    settings.postprocess(&repo);

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
            Some(("qif", sub)) => {
                export_qif(
                    &mut repo,
                    Path::new(
                        sub.get_one::<String>("output").expect("required"),
                    ),
                )?;
            }
            _ => unreachable!(),
        },
        Some(("networth", args)) => {
            networth(&mut repo, &settings, args)?;
        }
        Some(("cashflow", args)) => {
            cashflow(&mut repo, &settings, args)?;
        }
        Some(("stats", _)) => {
            stats(&repo, &settings)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
