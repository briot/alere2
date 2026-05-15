use alere_lib::times::{Instant, Intv};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Manage your finances
#[derive(Parser)]
#[command(version = "0.1", flatten_help = true, arg_required_else_help = true)]
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
        ///
        /// Total networth:
        ///     upto <date>      Total networth as of date
        ///     up to <date>
        ///     <date>           Same as upto <date>
        /// Networth evolution:
        ///     ytd                 Year to Date
        ///     <date>..<date>
        ///     <year>              Specific year
        ///     m0                  Current month
        ///     m1                  Previous month
        ///     y0                  Current year
        ///     y1                  Previous year
        ///     7d                  Over 7 days
        ///     3m                  Over 3 months
        ///     2y                  Over 2 years
        ///
        /// <date> can be one of:
        ///     epoch               Since the start of times
        ///     now
        ///     end                 Until forever
        ///     <year>              January 1st on that year
        ///     <year-month>        First day of the month
        ///     <year-month-day>    Specific day
        ///     yesterday
        ///     last month          Same day last month
        ///     last year
        ///     N days ago
        ///     start of <date>
        ///     end of <date>
        #[arg(
            short,
            long,
            value_delimiter = ',',
            default_value = "now,1y,ytd",
            verbatim_doc_comment
        )]
        periods: Vec<Intv>,

        /// Show rows with zero values
        #[arg(long)]
        show_zero: bool,

        /// Hide rows where no value has changed
        #[arg(long)]
        hide_all_same: bool,

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
        #[arg(short, long, value_delimiter = ',', default_value = "1m")]
        periods: Vec<Intv>,

        /// Show rows with zero values
        #[arg(long)]
        show_zero: bool,

        /// Hide rows where no value has changed
        #[arg(long)]
        hide_all_same: bool,

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
        file: Option<PathBuf>,
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

        /// Filter transactions by matching any column (supports * wildcard)
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Manage accounts
    Accounts {
        #[command(subcommand)]
        command: AccountsCommand,
    },

    /// Update stock prices and show networth changes
    Update,

    /// Show account balance history over time
    History {
        /// Account name filter (partial match)
        #[arg(long)]
        account: Option<String>,

        /// Time granularity: daily, monthly, yearly
        #[arg(long, default_value = "monthly")]
        granularity: String,

        /// Start date (e.g., "2020-01-01", "2y", "start of 2023")
        #[arg(long)]
        since: Option<String>,

        /// End date (defaults to now)
        #[arg(long)]
        before: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum AccountsCommand {
    /// List all accounts
    List {
        /// Filter accounts by partial name match (case-insensitive)
        #[arg(short, long)]
        filter: Option<String>,
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
