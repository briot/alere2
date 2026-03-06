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
    Metrics,

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
        #[command(flatten)]
        settings: crate::networth_view::Settings,
    },

    /// Show cashflow
    Cashflow {
        /// Columns to display (e.g 1y or 2m..now)
        #[arg(short, long, value_delimiter = ',')]
        periods: Vec<Intv>,
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
