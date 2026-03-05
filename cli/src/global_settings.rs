use alere_lib::{
    commodities::Commodity,
    formatters::{Formatter, Negative, Separators, SymbolQuote, Zero},
    repositories::Repository,
};
use chrono::{DateTime, Local};
use clap::Parser;

#[derive(Parser)]
pub struct GlobalSettings {
    /// Show market values with this currency
    #[arg(long = "currency", global = true)]
    pub commodity_str: Option<String>,

    /// Show rows with only zero values
    #[arg(long, global = true)]
    pub empty: bool,

    #[clap(skip)]
    pub commodity: Option<Commodity>,

    #[clap(skip)]
    pub format: alere_lib::formatters::Formatter,

    #[clap(skip)]
    pub reftime: DateTime<Local>,
}

impl GlobalSettings {
    pub fn postprocess(&mut self, repo: &Repository) {
        self.commodity = self
            .commodity_str
            .as_ref()
            .and_then(|m| repo.commodities.find(m));
        self.format.hide_symbol_if = self.commodity.clone();

        if let Some(c) = &self.commodity_str
            && self.commodity.is_none()
        {
            panic!("Unknown commodity {}", c);
        }
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        GlobalSettings {
            commodity_str: None,
            commodity: None,
            reftime: Local::now(),
            empty: false,
            format: Formatter {
                quote_symbol: SymbolQuote::UnquotedSymbol,
                hide_symbol_if: None,
                negative: Negative::MinusSign,
                separators: Separators::Every3Digit(','),
                comma: '.',
                negate: false,
                zero: Zero::Replace("0"),
            },
        }
    }
}
