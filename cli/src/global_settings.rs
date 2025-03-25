use alere_lib::{commodities::Commodity, repositories::Repository};
use chrono::{DateTime, Local};
use clap::{arg, Arg, ArgAction, ArgMatches};

pub struct GlobalSettings {
    pub commodity_str: Option<String>,
    pub commodity: Option<Commodity>,
    pub table: crate::tables::Settings,

    // How to display numbers
    pub format: alere_lib::formatters::Formatter,

    // Reference time for all relative dates ("a year ago").
    pub reftime: DateTime<Local>,

    // If true, a zero value is displayed as an empty cell instead
    pub hide_zero: bool,
}

impl GlobalSettings {
    /// Return the command line switches to configure the global settings
    pub fn cli() -> impl IntoIterator<Item = Arg> {
        [
            arg!(--currency [CURRENCY] "Show market values with this currency")
                .global(true),
            arg!(--empty "Show rows with only zero values")
                .action(ArgAction::SetTrue),
        ]
    }

    /// Create the settings from the command line arguments.  This creates
    /// the fields that are necessary for parsing the repository, but some
    /// fields can only be computed later once the repository has been loaded.
    pub fn new(args: &ArgMatches) -> Self {
        GlobalSettings {
            commodity_str: args.get_one::<String>("currency").cloned(),
            commodity: None,
            hide_zero: !args.get_flag("empty"),
            reftime: Local::now(),
            format: alere_lib::formatters::Formatter::default(),
            table: crate::tables::Settings {
                colsep: "│".to_string(),
                indent_size: 2,
            },
        }
    }

    /// Compute the remaining fields, after parsing the repository.
    pub fn postprocess(&mut self, repo: &Repository) {
        self.commodity = self
            .commodity_str
            .as_ref()
            .and_then(|m| repo.commodities.find(m));

        match &self.commodity_str {
            None => {}
            Some(c) => {
                if self.commodity.is_none() {
                    panic!("Unknown commodity {}", c);
                }
            }
        }
    }
}
