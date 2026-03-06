use alere_lib::{
    commodities::Commodity,
    formatters::{Formatter, Negative, Separators, SymbolQuote, Zero},
    repositories::Repository,
};
use chrono::{DateTime, Local};
use clap::{Parser, ValueEnum};
use tabled::settings::Style;

const MIN_COLUMN_WIDTH: usize = 5;

#[derive(Clone, ValueEnum)]
pub enum TableStyle {
    Modern,
    Markdown,
    Ascii,
    Sharp,
    Rounded,
    Extended,
    Psql,
    ReStructuredText,
    Dots,
    Blank,
}

impl TableStyle {
    pub fn apply(&self, table: &mut tabled::Table) {
        match self {
            TableStyle::Modern => table.with(Style::modern()),
            TableStyle::Markdown => table.with(Style::markdown()),
            TableStyle::Ascii => table.with(Style::ascii()),
            TableStyle::Sharp => table.with(Style::sharp()),
            TableStyle::Rounded => table.with(Style::rounded()),
            TableStyle::Extended => table.with(Style::extended()),
            TableStyle::Psql => table.with(Style::psql()),
            TableStyle::ReStructuredText => {
                table.with(Style::re_structured_text())
            }
            TableStyle::Dots => table.with(Style::dots()),
            TableStyle::Blank => table.with(Style::blank()),
        };
    }
}

pub fn limit_table_width(table: &mut tabled::Table, text_column: usize) {
    use tabled::settings::{Modify, Width, object::Columns};

    let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size()
    else {
        return;
    };
    let width = w as usize;

    let table_str = table.to_string();
    let max_line_width = table_str.lines().map(|l| l.len()).max().unwrap_or(0);

    if max_line_width <= width {
        return;
    }

    for text_width in (MIN_COLUMN_WIDTH..=30).rev() {
        let mut test_table = table.clone();
        test_table.with(
            Modify::new(Columns::single(text_column))
                .with(Width::truncate(text_width).suffix("...")),
        );

        let table_str = test_table.to_string();
        let current_width =
            table_str.lines().map(|l| l.len()).max().unwrap_or(0);

        if current_width <= width {
            *table = test_table;
            return;
        }
    }

    table.with(
        Modify::new(Columns::single(text_column))
            .with(Width::truncate(MIN_COLUMN_WIDTH).suffix("...")),
    );
}

#[derive(Parser)]
pub struct GlobalSettings {
    /// Show market values with this currency
    #[arg(long = "currency", global = true)]
    pub commodity_str: Option<String>,

    /// Show rows with only zero values
    #[arg(long, global = true)]
    pub empty: bool,

    /// Table style
    #[arg(long, global = true, default_value = "modern")]
    pub style: TableStyle,

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
            style: TableStyle::Modern,
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
