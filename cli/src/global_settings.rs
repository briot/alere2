use alere_lib::{commodities::CommodityId, repositories::Repository};
use clap::{arg, Arg, ArgMatches};
use chrono::{DateTime, Local};

pub struct GlobalSettings {
    pub commodity_str: Option<String>,
    pub commodity: Option<CommodityId>,
    pub table: crate::tables::Settings,

    // How to display numbers
    pub format: alere_lib::formatters::Formatter,

    // Reference time for all relative dates ("a year ago").
    pub reftime: DateTime<Local>,
}

impl GlobalSettings {
    /// Return the command line switches to configure the global settings
    pub fn cli() -> impl IntoIterator<Item = Arg> {
        [
            arg!(--currency [CURRENCY] "Show market values with this currency")
                .global(true),
        ]
    }

    /// Create the settings from the command line arguments.  This creates
    /// the fields that are necessary for parsing the repository, but some
    /// fields can only be computed later once the repository has been loaded.
    pub fn new(args: &ArgMatches) -> Self {
        GlobalSettings {
            commodity_str: args.get_one::<String>("currency").cloned(),
            commodity: None,
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
