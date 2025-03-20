use clap::{arg, Arg, Command};
use crate::global_settings::GlobalSettings;

pub(crate) fn build_cli() -> Command {
    Command::new("alere")
        .version("0.1")
        .about("Manage your finances")
        .subcommand_required(true)
        .subcommand_precedence_over_arg(true) // --x val1 val2 subcommand
        .flatten_help(true) // show help for all subcommands
        .arg_required_else_help(true) // show full help if nothing given
        .args(GlobalSettings::cli())
        .subcommand(Command::new("stats").about("Show statistics"))
        .subcommand(
            // Use    eval "$(alere completions zsh)"
            Command::new("completions")
                .about("Generate shell completions")
                .arg(
                    Arg::new("shell")
                        .value_name("SHELL")
                        .help("The shell to generate the completions for")
                        .required(true)
                        .value_parser(clap::builder::EnumValueParser::<
                            clap_complete_command::Shell,
                        >::new()),
                ),
        )
        .subcommand(
            Command::new("export")
                .about("Export data to other formats")
                .subcommand_required(true)
                .flatten_help(true)
                .subcommand(
                    Command::new("hledger").arg(
                        arg!(-o --output [FILE] "Name of output file")
                            .default_value("hledger.journal"),
                    ),
                )
                .subcommand(
                    Command::new("qif").arg(
                        arg!(-o --output [FILE] "Name of output file"),
                    ),
                ),
        )
        .subcommand(
            Command::new("networth")
                .about("Show current networth")
                .args(crate::networth_view::Settings::cli()),
        )
        .subcommand(Command::new("cashflow").about("Show cashflow"))
}
