mod args;
mod global_settings;
mod metrics_view;
mod networth_view;
mod perfs_view;

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
    multi_values::{MultiValue, Operation},
    networth::GroupBy,
    repositories::Repository,
    times::{Instant, Intv},
};
use anyhow::Result;
use chrono::Local;
use clap::{CommandFactory, Parser};
use futures::executor::block_on;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
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
fn metrics(repo: &Repository, globals: &GlobalSettings, periods: Vec<Intv>) -> Result<()> {
    let output = metrics_view(repo, globals, periods)?;
    println!("{}", output);
    Ok(())
}

/// Display stock performance
fn perfs(
    repo: &Repository,
    globals: &GlobalSettings,
    columns: Option<Vec<crate::perfs_view::PerfColumn>>,
) -> Result<()> {
    let output = perfs_view(repo, globals, columns)?;
    println!("{}", output);
    Ok(())
}

/// Show networth
fn networth(
    repo: &mut Repository,
    globals: &GlobalSettings,
    periods: Vec<Intv>,
    show_zero: bool,
    show_all_same: bool,
    no_subtotals: bool,
    no_elide: bool,
    delta: bool,
    delta_to_last: bool,
    price: bool,
    percent: bool,
) -> Result<()> {
    let output = networth_view(
        repo,
        |acc| acc.get_kind().is_networth(),
        globals,
        alere_lib::networth::Settings {
            hide_zero_rows: !show_zero,
            hide_all_same: !show_all_same,
            group_by: GroupBy::ParentAccount,
            subtotals: !no_subtotals,
            commodity: globals.commodity.clone(),
            elide_boring_accounts: !no_elide,
            intervals: periods,
        },
        &crate::networth_view::Settings {
            column_value: true,
            column_delta: delta,
            column_delta_to_last: delta_to_last,
            column_price: price,
            column_percent: percent,
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
    show_zero: bool,
    show_all_same: bool,
    no_subtotals: bool,
    no_elide: bool,
    delta: bool,
    delta_to_last: bool,
    price: bool,
    percent: bool,
) -> Result<()> {
    globals.format.negate = true;

    let income_expenses = networth_view(
        repo,
        |acc| acc.get_kind().is_expense() || acc.get_kind().is_income(),
        globals,
        alere_lib::networth::Settings {
            hide_zero_rows: !show_zero,
            hide_all_same: !show_all_same,
            group_by: GroupBy::ParentAccount,
            subtotals: !no_subtotals,
            commodity: globals.commodity.clone(),
            elide_boring_accounts: !no_elide,
            intervals: periods.to_vec(),
        },
        &crate::networth_view::Settings {
            column_value: true,
            column_delta: delta,
            column_delta_to_last: delta_to_last,
            column_price: price,
            column_percent: percent,
            account_names: AccountNameDepth::basename(),
        },
    );
    println!("{}", income_expenses.unwrap());
    Ok(())
}

fn ledger(
    repo: &Repository,
    settings: &GlobalSettings,
    account_filter: Option<&str>,
    short_name: bool,
    columns: Option<&Vec<String>>,
    since: Option<&Instant>,
    before: Option<&Instant>,
) -> Result<()> {
    use tabled::{Table, Tabled};

    // Parse column options
    let show_balance = columns.as_ref().map_or(true, |cols| {
        cols.iter().any(|c| c.eq_ignore_ascii_case("balance"))
    });
    let show_memo = columns.as_ref().map_or(true, |cols| {
        cols.iter().any(|c| c.eq_ignore_ascii_case("memo"))
    });
    let show_splits = columns.as_ref().map_or(true, |cols| {
        cols.iter().any(|c| c.eq_ignore_ascii_case("splits"))
    });

    // Parse date filters
    let since_date = since.and_then(|i| i.to_time(settings.reftime).ok());
    let before_date = before.and_then(|i| i.to_time(settings.reftime).ok());

    #[derive(Tabled)]
    struct LedgerRow {
        #[tabled(rename = "Date")]
        date: String,
        #[tabled(rename = "Account")]
        account: String,
        #[tabled(rename = "Amount")]
        amount: String,
        #[tabled(rename = "Balance")]
        balance: String,
        #[tabled(rename = "Memo")]
        memo: String,
    }

    let mut transactions: Vec<_> = repo.transactions().iter().collect();
    transactions.sort_by_key(|tx| {
        tx.splits().first().map(|s| s.post_ts).unwrap_or_default()
    });

    let display_depth = if short_name {
        AccountNameDepth::basename()
    } else {
        AccountNameDepth::unlimited()
    };

    let mut rows = Vec::new();
    let mut running_total = MultiValue::default();

    for tx in transactions {
        let splits = tx.splits();
        let memo = tx.memo();
        
        // Always match against full name
        let matches = if let Some(filter) = account_filter {
            splits.iter().any(|s| {
                s.account.name(AccountNameDepth::unlimited()).to_lowercase().contains(&filter.to_lowercase())
            })
        } else {
            true
        };

        if !matches {
            continue;
        }

        // Find the split for the filtered account (or first split if no filter)
        let main_idx = if let Some(filter) = account_filter {
            splits.iter().position(|s| {
                s.account.name(AccountNameDepth::unlimited()).to_lowercase().contains(&filter.to_lowercase())
            }).unwrap()
        } else {
            0
        };

        let main_split = &splits[main_idx];

        // Check date filters (but still update running total for all transactions)
        let in_date_range = {
            let ts = main_split.post_ts;
            let after_since = since_date.map_or(true, |d| ts >= d);
            let before_limit = before_date.map_or(true, |d| ts <= d);
            after_since && before_limit
        };

        // Update running total
        let amount_mv = match &main_split.operation {
            Operation::Credit(v) => v.clone(),
            Operation::BuyAmount { qty, .. } => MultiValue::new(qty.amount, &qty.commodity),
            Operation::BuyPrice { qty, .. } => MultiValue::new(qty.amount, &qty.commodity),
            Operation::AddShares { qty } => MultiValue::new(qty.amount, &qty.commodity),
            Operation::Reinvest { shares, .. } => shares.clone(),
            Operation::Dividend | Operation::Split { .. } => MultiValue::default(),
        };
        running_total += &amount_mv;

        // Skip display if outside date range
        if !in_date_range {
            continue;
        }

        // Convert balance to target currency if specified
        let balance_str = if show_balance {
            if settings.commodity.is_some() {
                let mut prices = repo.market_prices(settings.commodity.clone());
                let converted = prices.convert_multi_value(&running_total, &main_split.post_ts);
                converted.display(&settings.format)
            } else {
                running_total.display(&settings.format)
            }
        } else {
            String::new()
        };

        // Main split row
        rows.push(LedgerRow {
            date: main_split.post_ts.format("%Y-%m-%d").to_string(),
            account: main_split.account.name(display_depth),
            amount: amount_mv.display(&settings.format),
            balance: balance_str,
            memo: if show_memo {
                memo.as_ref().map(|s| s.to_string()).unwrap_or_default()
            } else {
                String::new()
            },
        });

        // Other splits (indented)
        if show_splits {
            for (idx, split) in splits.iter().enumerate() {
                if idx == main_idx {
                    continue;
                }
                let amount_str = match &split.operation {
                    Operation::Credit(v) => v.display(&settings.format),
                    Operation::BuyAmount { qty, .. } => qty.display(&settings.format),
                    Operation::BuyPrice { qty, .. } => qty.display(&settings.format),
                    Operation::AddShares { qty } => qty.display(&settings.format),
                    Operation::Reinvest { shares, .. } => shares.display(&settings.format),
                    Operation::Dividend => "dividend".to_string(),
                    Operation::Split { ratio, .. } => format!("split {}", ratio),
                };
                rows.push(LedgerRow {
                    date: String::new(),
                    account: format!("  {}", split.account.name(display_depth)),
                    amount: amount_str,
                    balance: String::new(),
                    memo: String::new(),
                });
            }
        }
    }

    let mut table = Table::new(rows);
    settings.style.apply(&mut table);
    println!("{}", table);

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
            periods,
            show_zero,
            show_all_same,
            no_subtotals,
            no_elide,
            delta,
            delta_to_last,
            price,
            percent,
        } => {
            networth(
                repo,
                settings,
                periods.clone(),
                *show_zero,
                *show_all_same,
                *no_subtotals,
                *no_elide,
                *delta,
                *delta_to_last,
                *price,
                *percent,
            )?;
        }
        Commands::Cashflow {
            periods,
            show_zero,
            show_all_same,
            no_subtotals,
            no_elide,
            delta,
            delta_to_last,
            price,
            percent,
        } => {
            cashflow(
                repo,
                settings,
                periods,
                *show_zero,
                *show_all_same,
                *no_subtotals,
                *no_elide,
                *delta,
                *delta_to_last,
                *price,
                *percent,
            )?;
        }
        Commands::Metrics { periods } => {
            metrics(repo, settings, periods.clone())?;
        }
        Commands::Perf { columns } => {
            perfs(repo, settings, columns.clone())?;
        }
        Commands::Ledger { account, short_name, columns, since, before } => {
            ledger(repo, settings, account.as_deref(), *short_name, columns.as_ref(), since.as_ref(), before.as_ref())?;
        }
        Commands::Batch { file } => {
            let content = std::fs::read_to_string(file)?;
            let mut first = true;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if !first {
                    println!();
                }
                first = false;
                println!("=== {} ===", line);

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
                    style: settings.style.clone(),
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
    settings.reftime = Local::now();

    let multi = MultiProgress::new();
    let logger = env_logger::Builder::from_default_env().build();
    indicatif_log_bridge::LogWrapper::new(multi.clone(), logger)
        .try_init()
        .unwrap();

    let progress = multi.add(
        ProgressBar::new(1)
            .with_style(
                ProgressStyle::with_template(
                    "[{pos:2}/{len:2}] {msg} {wide_bar} {elapsed_precise}",
                )
                .unwrap(),
            )
            .with_message("importing kmy"),
    );

    let mut kmy = KmyMoneyImporter::default();
    let mut repo = block_on(kmy.import_file(&cli.input, |current, max| {
        progress.set_length(max);
        progress.set_position(current);
    }))?;
    progress.finish_and_clear();

    settings.postprocess(&repo);
    run_subcommand(&mut repo, &cli.command, &mut settings)
}
