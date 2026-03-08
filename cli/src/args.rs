use alere_lib::times::{Instant, Intv};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Manage your finances
#[derive(Parser)]
#[command(
    version = "0.1",
    flatten_help = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: crate::global_settings::GlobalSettings,

    /// Input file (KMyMoney format)
    #[arg(short, long, global = true, default_value = "./Comptes.kmy")]
    pub input: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show metrics
    Metrics {
        /// Periods to display (e.g 1y or 2m..now)
        #[arg(short, long, value_delimiter = ',', default_value = "1y,ytd")]
        periods: Vec<Intv>,
    },

    /// Show stock performance
    Perf {
        /// Columns to display (comma-separated)
        #[arg(long, value_delimiter = ',')]
        columns: Option<Vec<crate::perfs_view::PerfColumn>>,
    },

    /// Generate shell completions
    /// Use: eval "$(alere completions zsh)"
    Completions {
        /// The shell to generate the completions for
        shell: clap_complete_command::Shell,
    },

    /// Export data to other formats
    Export {
        #[command(subcommand)]
        format: ExportFormat,
    },

    /// Show current networth
    Networth {
        /// Periods to display (e.g 1y or 2m..now)
        #[arg(short, long, value_delimiter = ',', default_value = "1y,ytd")]
        periods: Vec<Intv>,

        /// Show rows with zero values
        #[arg(long)]
        show_zero: bool,

        /// Show rows where values haven't changed
        #[arg(long)]
        show_all_same: bool,

        /// Disable subtotals for parent accounts
        #[arg(long)]
        no_subtotals: bool,

        /// Don't collapse boring accounts
        #[arg(long)]
        no_elide: bool,

        /// Show delta column
        #[arg(long)]
        delta: bool,

        /// Show delta to last column
        #[arg(long)]
        delta_to_last: bool,

        /// Show price column
        #[arg(long)]
        price: bool,

        /// Show percent of total column
        #[arg(long)]
        percent: bool,
    },

    /// Show cashflow
    Cashflow {
        /// Columns to display (e.g 1y or 2m..now)
        #[arg(short, long, value_delimiter = ',')]
        periods: Vec<Intv>,

        /// Show rows with zero values
        #[arg(long)]
        show_zero: bool,

        /// Show rows where values haven't changed
        #[arg(long)]
        show_all_same: bool,

        /// Disable subtotals for parent accounts
        #[arg(long)]
        no_subtotals: bool,

        /// Don't collapse boring accounts
        #[arg(long)]
        no_elide: bool,

        /// Show delta column
        #[arg(long)]
        delta: bool,

        /// Show delta to last column
        #[arg(long)]
        delta_to_last: bool,

        /// Show price column
        #[arg(long)]
        price: bool,

        /// Show percent of total column
        #[arg(long)]
        percent: bool,
    },

    /// Run all commands found in the file (or stdin if not specified)
    Batch { 
        /// File containing commands (one per line). If not specified, reads from stdin.
        file: Option<PathBuf> 
    },

    /// Show ledger (transaction list) for accounts
    Ledger {
        /// Account name to filter (optional, shows all if not specified)
        #[arg(short, long)]
        account: Option<String>,

        /// Show only account basename instead of full path
        #[arg(long)]
        short_name: bool,

        /// Columns to display (comma-separated: balance, memo, splits)
        #[arg(long, value_delimiter = ',')]
        columns: Option<Vec<String>>,

        /// Only show transactions since this date (e.g., "3m", "2024")
        #[arg(long)]
        since: Option<Instant>,

        /// Only show transactions before this date (e.g., "now", "2024")
        #[arg(long)]
        before: Option<Instant>,
    },
}

#[derive(Subcommand)]
pub enum ExportFormat {
    /// Export to hledger format
    Hledger {
        /// Name of output file
        #[arg(short, long, default_value = "hledger.journal")]
        output: String,
    },
}
