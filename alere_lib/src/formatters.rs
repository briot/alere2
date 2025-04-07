use crate::commodities::Commodity;
use rust_decimal::{Decimal, RoundingStrategy};

/// How to display commodities
#[derive(Clone, Copy, Default)]
pub enum SymbolQuote {
    #[default]
    UnquotedSymbol, // e.g. $   Will be displayed before or after the value
    UnquotedName,          // The name of the commodity (e.g. USD)
    QuotedNameIfSpecial,   //  only if it contains spaces, starts with digit,...
    QuotedNameAlways,      // e.g.  "USD" or "US Dollars"
    QuotedSymbolIfSpecial, //  only if it contains spaces, starts with digit,...
    QuotedSymbolAlways,    // e.g.  "$"
}

/// How to display negative values
#[derive(Clone, Copy, Default)]
pub enum Negative {
    #[default]
    MinusSign, // USD -123
    Parenthesis,  // USD (123)
    SeparateSign, // -USD 123
}

/// How to display large numbers
#[derive(Clone, Copy)]
pub enum Separators {
    None,              // no special formatting    1234456.789
    Every3Digit(char), // char every 3 digits      1,234,456.789
}
impl Default for Separators {
    fn default() -> Self {
        Separators::Every3Digit(',')
    }
}

/// How to display zero values
#[derive(Clone, Copy)]
pub enum Zero {
    Empty,                 // display nothing
    Replace(&'static str), // display a specific text instead (e.g. "-")
}

pub struct Formatter {
    pub quote_symbol: SymbolQuote,
    pub hide_symbol_if: Option<Commodity>,
    pub negative: Negative,
    pub separators: Separators,
    pub comma: char,
    pub zero: Zero,
    pub negate: bool,  // display opposite sign
    // ??? support for printing currencies as EUR rather than the symbol
    // (non-unicode)
    // ??? support for color
}

impl Default for Formatter {
    fn default() -> Self {
        Self {
            comma: '.',
            quote_symbol: SymbolQuote::default(),
            hide_symbol_if: None,
            negative: Negative::default(),
            separators: Separators::default(),
            zero: Zero::Empty,
            negate: false,
        }
    }
}

impl Formatter {
    /// Display the absolute value of value
    fn push_abs_num(&self, into: &mut String, value: Decimal, precision: u8) {
        let mut rounded = value.abs().round_dp_with_strategy(
            precision as u32,
            RoundingStrategy::MidpointTowardZero,
        );

        if self.negate {
            rounded = -rounded;
        }

        match self.separators {
            Separators::None => {
                into.push_str(&rounded.to_string());
            }
            Separators::Every3Digit(sep) => {
                let val: Vec<char> = rounded.to_string().chars().collect();
                let decimal =
                    val.iter().position(|&r| r == '.').unwrap_or(val.len());

                for (idx, p) in val[0..decimal].iter().enumerate() {
                    if idx > 0 && (decimal - idx) % 3 == 0 {
                        into.push(sep);
                    }
                    into.push(*p);
                }

                if precision > 0 {
                    into.push(self.comma);
                    let mut count = 0_u8;
                    for p in val.iter().skip(decimal + 1) {
                        into.push(*p);
                        count += 1;
                    }
                    for _ in count + 1..=precision {
                        into.push('0');
                    }
                }
            }
        }
    }

    fn push_commodity(&self, into: &mut String, commodity: &Commodity) {
        match self.quote_symbol {
            SymbolQuote::UnquotedSymbol => {
                into.push_str(&commodity.get_symbol());
            }
            SymbolQuote::UnquotedName => {
                into.push_str(&commodity.get_name());
            }
            SymbolQuote::QuotedSymbolIfSpecial => {
                let symbol = &commodity.get_symbol();
                if symbol.chars().all(|c| c.is_alphanumeric()) {
                    into.push_str(symbol);
                } else {
                    into.push('"');
                    into.push_str(symbol);
                    into.push('"');
                }
            }
            SymbolQuote::QuotedSymbolAlways => {
                into.push('"');
                into.push_str(&commodity.get_symbol());
                into.push('"');
            }
            SymbolQuote::QuotedNameIfSpecial => {
                let name = &commodity.get_name();
                if name.chars().all(|c| c.is_alphanumeric()) {
                    into.push_str(name);
                } else {
                    into.push('"');
                    into.push_str(name);
                    into.push('"');
                }
            }
            SymbolQuote::QuotedNameAlways => {
                into.push('"');
                into.push_str(&commodity.get_name());
                into.push('"');
            }
        }
    }

    pub fn display_symbol(&self, com: &Commodity) -> String {
        let mut buffer = String::new();
        self.push_commodity(&mut buffer, com);
        buffer
    }

    pub fn push_from_commodity(
        &self,
        into: &mut String,
        value: Decimal,
        commodity: &Commodity,
    ) {
        self.push(into, value, commodity);
    }

    pub fn display_from_commodity(
        &self,
        value: Decimal,
        commodity: &Commodity,
    ) -> String {
        let mut buffer = String::new();
        self.push_from_commodity(&mut buffer, value, commodity);
        buffer
    }

    pub fn display(&self, value: Decimal, comm: &Commodity) -> String {
        let mut buffer = String::new();
        self.push(&mut buffer, value, comm);
        buffer
    }

    pub fn push_zero(&self, into: &mut String) {
        match self.zero {
            Zero::Empty => {}
            Zero::Replace(z) => into.push_str(z),
        }
    }

    pub fn push(&self, into: &mut String, value: Decimal, comm: &Commodity) {
        if value.is_zero() {
            self.push_zero(into);
            return;
        }

        let precision = comm.get_display_precision();

        if let Some(hide) = &self.hide_symbol_if {
            if hide == comm {
                self.push_abs_num(into, value, precision);
                return;
            }
        }

        let symbol_after = comm.symbol_after();
        if !symbol_after {
            if value.is_sign_negative() {
                match self.negative {
                    Negative::SeparateSign => {
                        into.push('-');
                        self.push_commodity(into, comm);
                        into.push(' ');
                        self.push_abs_num(into, value, precision);
                    }
                    Negative::MinusSign => {
                        self.push_commodity(into, comm);
                        into.push(' ');
                        into.push('-');
                        self.push_abs_num(into, value, precision);
                    }
                    Negative::Parenthesis => {
                        self.push_commodity(into, comm);
                        into.push(' ');
                        into.push('(');
                        self.push_abs_num(into, value, precision);
                        into.push(')');
                    }
                }
            } else {
                self.push_commodity(into, comm);
                into.push(' ');
                self.push_abs_num(into, value, precision);
            }
        } else if value.is_sign_negative() {
            match self.negative {
                Negative::SeparateSign | Negative::MinusSign => {
                    into.push('-');
                    self.push_abs_num(into, value, precision);
                }
                Negative::Parenthesis => {
                    into.push('(');
                    self.push_abs_num(into, value, precision);
                    into.push(')');
                }
            }
            into.push(' ');
            self.push_commodity(into, comm);
        } else {
            self.push_abs_num(into, value, precision);
            into.push(' ');
            self.push_commodity(into, comm);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::commodities::test::create_currency;
    use crate::commodities::CommodityCollection;
    use crate::formatters::{Formatter, Negative, Separators, SymbolQuote};
    use rust_decimal_macros::dec;

    #[test]
    fn test_display() {
        let f = Formatter::default();
        let mut cc = CommodityCollection::default();
        let eur_after = create_currency(&mut cc, "EUR", 2, true);
        let eur_before = create_currency(&mut cc, "EUR", 2, false);
        let eur_before_3 = create_currency(&mut cc, "EUR", 3, false);
        let eur_before_4 = create_currency(&mut cc, "EUR", 4, false);
        let mysym_after = create_currency(&mut cc, "MY SYMB", 2, true);

        // check no leading ',' is added
        assert_eq!(
            f.display(dec!(234567), &eur_after),
            "234,567.00 EUR"
        );

        assert_eq!(
            f.display(dec!(1234567.238), &eur_after),
            "1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(1234567.238), &eur_before_3),
            "EUR 1,234,567.238"
        );
        assert_eq!(
            f.display(dec!(1234567.238), &eur_before_4),
            "EUR 1,234,567.2380"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_after),
            "-1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_before),
            "EUR -1,234,567.24"
        );

        // Check rounding
        assert_eq!(f.display(dec!(0.234), &eur_after), "0.23 EUR");
        assert_eq!(f.display(dec!(0.235), &eur_after), "0.23 EUR");

        // round to nearest even
        assert_eq!(f.display(dec!(0.245), &eur_after), "0.24 EUR");
        assert_eq!(f.display(dec!(1.00), &eur_after), "1.00 EUR");
        assert_eq!(f.display(dec!(1), &eur_after), "1.00 EUR");

        let f = Formatter {
            quote_symbol: SymbolQuote::QuotedSymbolIfSpecial,
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(1234567.238), &eur_after),
            "1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(1234567.238), &mysym_after),
            "1,234,567.24 \"MY SYMB\""
        );

        let f = Formatter {
            negative: Negative::Parenthesis,
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_after),
            "(1,234,567.24) EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_before),
            "EUR (1,234,567.24)"
        );

        let f = Formatter {
            negative: Negative::SeparateSign,
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_after),
            "-1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_before),
            "-EUR 1,234,567.24"
        );

        let f = Formatter {
            comma: ',',
            separators: Separators::Every3Digit(' '),
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(1234567.238), &eur_after),
            "1 234 567,24 EUR"
        );
        assert_eq!(
            f.display(dec!(1234567.238), &eur_before),
            "EUR 1 234 567,24"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_after),
            "-1 234 567,24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_before),
            "EUR -1 234 567,24"
        );

        let f = Formatter {
            separators: Separators::None,
            ..Formatter::default()
        };
        assert_eq!(f.display(dec!(1234567.238), &eur_after), "1234567.24 EUR");
        assert_eq!(f.display(dec!(1234567.238), &eur_before), "EUR 1234567.24");
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_after),
            "-1234567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), &eur_before),
            "EUR -1234567.24"
        );
    }
}
