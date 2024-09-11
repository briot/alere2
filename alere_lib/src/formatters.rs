use crate::commodities::Commodity;
use rust_decimal::{Decimal, RoundingStrategy};

/// Whether the name of commodities should be quotes
#[derive(Clone, Copy, Default)]
pub enum SymbolQuote {
    #[default]
    NeverQuote,
    QuoteSpecial, //  only if it contains spaces, starts with digit,...
    QuoteAlways,
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
    None,
    Every3Digit(char), // char every 3 digits      1,234,456.789
}
impl Default for Separators {
    fn default() -> Self {
        Separators::Every3Digit(',')
    }
}

pub struct Formatter {
    pub quote_symbol: SymbolQuote,
    pub negative: Negative,
    pub separators: Separators,
    pub comma: char,
    pub zero: &'static str,
}

impl Default for Formatter {
    fn default() -> Self {
        Self {
            comma: '.',
            quote_symbol: SymbolQuote::default(),
            negative: Negative::default(),
            separators: Separators::default(),
            zero: "",
        }
    }
}

impl Formatter {
    /// Display the absolute value of value
    fn push_abs_num(&self, into: &mut String, value: Decimal, precision: u8) {
        let rounded = value.abs().round_dp_with_strategy(
            precision as u32,
            RoundingStrategy::MidpointTowardZero,
        );

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

    fn push_symbol(&self, into: &mut String, symbol: &str) {
        match self.quote_symbol {
            SymbolQuote::NeverQuote => {
                into.push_str(symbol);
            }
            SymbolQuote::QuoteSpecial => {
                if symbol.contains(' ')
                    || symbol.contains('-')
                    || symbol.contains('+')
                    || symbol.contains('.')
                    || symbol
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(true)
                {
                    into.push('"');
                    into.push_str(symbol);
                    into.push('"');
                } else {
                    into.push_str(symbol);
                }
            }
            SymbolQuote::QuoteAlways => {
                into.push('"');
                into.push_str(symbol);
                into.push('"');
            }
        }
    }

    pub fn display_symbol(&self, symbol: &str) -> String {
        let mut buffer = String::new();
        self.push_symbol(&mut buffer, symbol);
        buffer
    }

    pub fn push_from_commodity(
        &self,
        into: &mut String,
        value: Decimal,
        commodity: &Commodity,
    ) {
        self.push(
            into,
            value,
            &commodity.symbol,
            commodity.symbol_after,
            commodity.display_precision,
        );
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

    pub fn display(
        &self,
        value: Decimal,
        symbol: &str,
        symbol_after: bool,
        precision: u8,
    ) -> String {
        let mut buffer = String::new();
        self.push(&mut buffer, value, symbol, symbol_after, precision);
        buffer
    }

    pub fn push(
        &self,
        into: &mut String,
        value: Decimal,
        symbol: &str,
        symbol_after: bool,
        precision: u8,
    ) {
        if value.is_zero() {
            into.push_str(self.zero);
            return;
        }
        if !symbol_after {
            if value.is_sign_negative() {
                match self.negative {
                    Negative::SeparateSign => {
                        into.push('-');
                        self.push_symbol(into, symbol);
                        into.push(' ');
                        self.push_abs_num(into, value, precision);
                    }
                    Negative::MinusSign => {
                        self.push_symbol(into, symbol);
                        into.push(' ');
                        into.push('-');
                        self.push_abs_num(into, value, precision);
                    }
                    Negative::Parenthesis => {
                        self.push_symbol(into, symbol);
                        into.push(' ');
                        into.push('(');
                        self.push_abs_num(into, value, precision);
                        into.push(')');
                    }
                }
            } else {
                self.push_symbol(into, symbol);
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
        } else {
            self.push_abs_num(into, value, precision);
        }

        if symbol_after {
            into.push(' ');
            self.push_symbol(into, symbol);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::formatters::{Formatter, Negative, Separators, SymbolQuote};
    use rust_decimal_macros::dec;

    #[test]
    fn test_display() {
        let f = Formatter::default();
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", true, 2),
            "1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", false, 3),
            "EUR 1,234,567.238"
        );
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", false, 4),
            "EUR 1,234,567.2380"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", true, 2),
            "-1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", false, 2),
            "EUR -1,234,567.24"
        );

        // Check rounding
        assert_eq!(f.display(dec!(0.234), "EUR", true, 2), "0.23 EUR");
        assert_eq!(f.display(dec!(0.235), "EUR", true, 2), "0.23 EUR");

        // round to nearest even
        assert_eq!(f.display(dec!(0.245), "EUR", true, 2), "0.24 EUR");
        assert_eq!(f.display(dec!(1.00), "EUR", true, 2), "1.00 EUR");
        assert_eq!(f.display(dec!(1), "EUR", true, 2), "1.00 EUR");

        let f = Formatter {
            quote_symbol: SymbolQuote::QuoteSpecial,
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", true, 2),
            "1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(1234567.238), "MY SYMB", true, 2),
            "1,234,567.24 \"MY SYMB\""
        );

        let f = Formatter {
            negative: Negative::Parenthesis,
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", true, 2),
            "(1,234,567.24) EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", false, 2),
            "EUR (1,234,567.24)"
        );

        let f = Formatter {
            negative: Negative::SeparateSign,
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", true, 2),
            "-1,234,567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", false, 2),
            "-EUR 1,234,567.24"
        );

        let f = Formatter {
            comma: ',',
            separators: Separators::Every3Digit(' '),
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", true, 2),
            "1 234 567,24 EUR"
        );
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", false, 2),
            "EUR 1 234 567,24"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", true, 2),
            "-1 234 567,24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", false, 2),
            "EUR -1 234 567,24"
        );

        let f = Formatter {
            separators: Separators::None,
            ..Formatter::default()
        };
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", true, 2),
            "1234567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(1234567.238), "EUR", false, 2),
            "EUR 1234567.24"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", true, 2),
            "-1234567.24 EUR"
        );
        assert_eq!(
            f.display(dec!(-1234567.238), "EUR", false, 2),
            "EUR -1234567.24"
        );
    }
}
