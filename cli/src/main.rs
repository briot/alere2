mod args;
mod global_settings;
mod metrics_view;
mod networth_view;
mod perfs_view;
pub mod tables;

use crate::{
    args::build_cli, global_settings::GlobalSettings,
    metrics_view::metrics_view, networth_view::networth_view,
    perfs_view::perfs_view,
};
use alere_lib::{
    accounts::AccountNameDepth,
    formatters::{Formatter, SymbolQuote, Zero},
    hledger::Hledger,
    importers::{Exporter, Importer},
    kmymoney::KmyMoneyImporter,
    networth::GroupBy,
    repositories::Repository,
    times::{Instant, Intv},
};
use anyhow::Result;
use clap::ArgMatches;
use futures::executor::block_on;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

/// Export all transaction to hledger format
fn export_hledger(repo: &mut Repository, output: &Path) -> Result<()> {
    let format = Formatter {
        quote_symbol: SymbolQuote::QuotedNameIfSpecial,
        zero: Zero::Replace("0"),
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

/// Display metrics
fn metrics(repo: &Repository, globals: &GlobalSettings) -> Result<()> {
    let output = metrics_view(repo, globals)?;
    println!("{}", output);
    Ok(())
}

/// Display stock performance
fn perfs(repo: &Repository, globals: &GlobalSettings) -> Result<()> {
    let output = perfs_view(repo, globals)?;
    println!("{}", output);
    Ok(())
}

/// Show networth
fn networth(
    repo: &mut Repository,
    globals: &GlobalSettings,
    args: &ArgMatches,
) -> Result<()> {
    let output = networth_view(
        repo,
        args,
        |acc| acc.get_kind().is_networth(),
        globals,
        alere_lib::networth::Settings {
            hide_zero_rows: !globals.empty,
            hide_all_same: false,
            group_by: GroupBy::ParentAccount,
            subtotals: true,
            commodity: globals.commodity.clone(),
            elide_boring_accounts: true,
            intervals: vec![
                Intv::UpTo(Instant::YearsAgo(1)),
                Intv::UpTo(Instant::MonthsAgo(1)),
                Intv::UpTo(Instant::Now),
            ],
        },
        &crate::networth_view::Settings {
            column_value: true,
            column_delta: false,
            column_delta_to_last: false,
            column_price: false,
            column_percent: false,
            account_names: AccountNameDepth::basename(),
        },
    )?;
    println!("{}", output);
    Ok(())
}

/// Show income-expenses
fn cashflow(
    repo: &mut Repository,
    globals: &mut GlobalSettings,
    args: &ArgMatches,
) -> Result<()> {
    globals.format.negate = true;

    let income_expenses = networth_view(
        repo,
        args,
        |acc| acc.get_kind().is_expense() || acc.get_kind().is_income(),
        globals,
        alere_lib::networth::Settings {
            hide_zero_rows: !globals.empty,
            hide_all_same: false,
            group_by: GroupBy::ParentAccount,
            subtotals: true,
            commodity: globals.commodity.clone(),
            elide_boring_accounts: true,
            intervals: vec![
                Intv::LastNYears(1),
                Intv::Monthly {
                    begin: Instant::MonthsAgo(5),
                    end: Instant::Now,
                },
                // Intv::LastNMonths(1),
            ],
        },
        &crate::networth_view::Settings {
            column_value: true,
            column_delta: false,
            column_delta_to_last: false,
            column_price: false,
            column_percent: false,
            account_names: AccountNameDepth::basename(),
        },
    );
    println!("{}", income_expenses.unwrap());
    Ok(())
}

fn run_subcommand(
    repo: &mut Repository,
    matches: &clap::ArgMatches,
    settings: &mut GlobalSettings,
) -> Result<()> {
    match matches.subcommand() {
        Some(("completions", args)) => {
            if let Some(shell) =
                args.get_one::<clap_complete_command::Shell>("shell")
            {
                let mut command = build_cli();
                shell.generate(&mut command, &mut std::io::stdout());
            }
        }
        Some(("export", sub)) => match sub.subcommand() {
            Some(("hledger", sub)) => {
                export_hledger(
                    repo,
                    Path::new(
                        sub.get_one::<String>("output").expect("required"),
                    ),
                )?;
            }
            _ => unreachable!(),
        },
        Some(("networth", args)) => {
            networth(repo, settings, args)?;
        }
        Some(("cashflow", args)) => {
            cashflow(repo, settings, args)?;
        }
        Some(("metrics", _)) => {
            metrics(repo, settings)?;
        }
        Some(("perf", _)) => {
            perfs(repo, settings)?;
        }
        Some(("batch", args)) => {
            let file: &PathBuf = args.get_one("file").unwrap();
            let content = std::fs::read_to_string(file)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let args = shlex::split(line).ok_or_else(|| {
                    anyhow::anyhow!("invalid quoting: {}", line)
                })?;
                let args = std::iter::once("alere".to_string()).chain(args);
                let matches = build_cli().try_get_matches_from(args)?;
                let mut global =
                    GlobalSettings::new_with_defaults(&matches, settings);
                global.postprocess(&repo);
                run_subcommand(repo, &matches, &mut global)?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = build_cli();
    let args = cli.get_matches();
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
    run_subcommand(&mut repo, &args, &mut settings)
}
