use alere_lib::times::Intv;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Manage your finances
#[derive(Parser)]
#[command(
    version = "0.1",
    subcommand_precedence_over_arg = true,
    flatten_help = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: crate::global_settings::GlobalSettings,

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

    /// Run all commands found in the file
    Batch { file: PathBuf },
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
