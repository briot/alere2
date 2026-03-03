mod args;
mod global_settings;
mod metrics_view;
mod networth_view;
mod perfs_view;
pub mod tables;

use crate::{
    args::{Cli, Commands, ExportFormat},
    global_settings::GlobalSettings,
    metrics_view::metrics_view,
    networth_view::networth_view,
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
use clap::{CommandFactory, Parser};
use futures::executor::block_on;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

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
    _settings: &crate::networth_view::Settings,
) -> Result<()> {
    let output = networth_view(
        repo,
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
    periods: &[Intv],
) -> Result<()> {
    globals.format.negate = true;

    let income_expenses = networth_view(
        repo,
        |acc| acc.get_kind().is_expense() || acc.get_kind().is_income(),
        globals,
        alere_lib::networth::Settings {
            hide_zero_rows: !globals.empty,
            hide_all_same: false,
            group_by: GroupBy::ParentAccount,
            subtotals: true,
            commodity: globals.commodity.clone(),
            elide_boring_accounts: true,
            intervals: periods.to_vec(),
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
    command: &Commands,
    settings: &mut GlobalSettings,
) -> Result<()> {
    match command {
        Commands::Completions { shell } => {
            shell.generate(&mut Cli::command(), &mut std::io::stdout());
        }
        Commands::Export { format } => match format {
            ExportFormat::Hledger { output } => {
                export_hledger(repo, Path::new(output))?;
            }
        },
        Commands::Networth {
            settings: networth_settings,
        } => {
            networth(repo, settings, networth_settings)?;
        }
        Commands::Cashflow { periods } => {
            cashflow(repo, settings, periods)?;
        }
        Commands::Metrics => {
            metrics(repo, settings)?;
        }
        Commands::Perf => {
            perfs(repo, settings)?;
        }
        Commands::Batch { file } => {
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
                let cli = Cli::try_parse_from(args)?;
                let mut global = GlobalSettings {
                    commodity_str: cli
                        .global
                        .commodity_str
                        .or(settings.commodity_str.clone()),
                    empty: cli.global.empty || settings.empty,
                    ..GlobalSettings::default()
                };
                global.postprocess(repo);
                run_subcommand(repo, &cli.command, &mut global)?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut settings = cli.global;
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
    run_subcommand(&mut repo, &cli.command, &mut settings)
}
