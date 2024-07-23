use crate::price_sources::PriceSourceId;
use rust_decimal::{Decimal, RoundingStrategy};

#[derive(Default)]
pub struct CommodityCollection {
    commodities: Vec<Commodity>,
    currencies: Vec<CommodityId>, // duplicates some of those in commodities
}

impl CommodityCollection {
    pub fn add(&mut self, commodity: Commodity) -> CommodityId {
        let is_currency = commodity.is_currency;
        self.commodities.push(commodity);
        let id = CommodityId(self.commodities.len() as u16);
        if is_currency {
            self.currencies.push(id);
        }
        id
    }

    pub fn get_mut(&mut self, id: CommodityId) -> Option<&mut Commodity> {
        self.commodities.get_mut(id.0 as usize - 1)
    }

    pub fn get(&self, id: CommodityId) -> Option<&Commodity> {
        self.commodities.get(id.0 as usize - 1)
    }

    pub fn list_currencies(&self) -> &[CommodityId] {
        &self.currencies
    }

    pub fn find(&self, name: &str) -> Option<CommodityId> {
        self.commodities
            .iter()
            .enumerate()
            .find(|(_, c)| c.name == name)
            .map(|(id, _)| CommodityId(id as u16 + 1))
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub struct CommodityId(pub u16);

impl CommodityId {
    pub fn inc(&self) -> CommodityId {
        CommodityId(self.0 + 1)
    }
}

/// Currencies, securities and any tangible asset accounted for in accounts.
///
/// All accounts (and the splits that apply to that account) have values given
/// in one or more commodities (currencies or securities).
///
/// However, one commodity might be traded in multiple accounts.  For instance,
/// if you have multiple investment portfolios, they could all be trading
/// COCA COLA for instance.  Each of them will have its own performance
/// statistics, depending on when you bought, the fees applied by the
/// institution, and so on.

#[derive(Debug)]
pub struct Commodity {
    /// Name as displayed in selection boxes in the GUI.  For instance, it
    /// could be "Euro", "Apple Inc.", ...
    pub name: String,

    /// Symbol to display the commodity. For instance, it could be the
    /// euro sign, or "AAPL".  "before" and "after" refer to whether the
    /// symbol is displayed before or after the numeric value.
    symbol_before: String,
    symbol_after: String,

    /// What kind of commodity this is.
    pub is_currency: bool,

    /// For online quotes.
    /// The source refers to one of the plugins available to download
    /// online information.
    ///
    /// The quote_symbol is the ticker, the ISIN, or the iso code for currencies.
    /// It is the information searched for in the online source.
    ///
    /// The quote_currency is the currency in which we retrieve the data,
    /// which is cached because fetching that information is slow in Yahoo.
    /// So if we start with the AAPL commodity,  quote_currency might be USD if
    /// the online source gives prices in USD.
    pub quote_symbol: Option<String>,
    pub quote_source: Option<PriceSourceId>,
    pub quote_currency: Option<CommodityId>,

    /// Number of digits in the fractional part
    pub display_precision: u8,
}

impl Commodity {
    pub fn new(
        name: &str,
        symbol_before: &str,
        symbol_after: &str,
        is_currency: bool,
        quote_symbol: Option<&str>,
        display_precision: u8,
    ) -> Self {
        Commodity {
            name: name.into(),
            symbol_before: symbol_before.trim().to_string(),
            symbol_after: symbol_after.trim().into(),
            is_currency,
            quote_symbol: quote_symbol.map(str::to_string),
            quote_source: None,
            quote_currency: None,
            display_precision,
        }
    }

    //  Display a given value for a commodity
    pub fn display(&self, value: &Decimal) -> String {
        format!(
            "{}{}{:.width$}{}{}",
            self.symbol_before,
            if self.symbol_before.is_empty() {
                ""
            } else {
                " "
            },
            value.round_dp_with_strategy(
                self.display_precision as u32,
                RoundingStrategy::MidpointTowardZero),
            if self.symbol_after.is_empty() {
                ""
            } else {
                " "
            },
            self.symbol_after,
            width=self.display_precision as usize,
        )
    }
}

#[cfg(test)]
mod test {
    use crate::commodities::{CommodityId, CommodityCollection, Commodity};
    use rust_decimal_macros::dec;

    pub fn create_currency(coll: &mut CommodityCollection, name: &str) -> CommodityId {
        let commodity = Commodity::new(
            name,
            "",
            name,
            true,
            None,
            2,
        );
        coll.add(commodity)
    }

    pub fn create_security(coll: &mut CommodityCollection, name: &str) -> CommodityId {
        let commodity = Commodity::new(
            name,
            "",
            name,
            false,
            None,
            2,
        );
        coll.add(commodity)
    }

    #[test]
    fn test_commodity() {
        let mut coll = CommodityCollection::default();
        let eur = create_currency(&mut coll, "EUR");
        let aapl = create_security(&mut coll, "AAPL");
        assert_eq!(coll.list_currencies(), &[eur]);
        assert_eq!(coll.find("EUR"), Some(eur));
        assert_eq!(coll.find("AAPL"), Some(aapl));
        assert_eq!(coll.find("FOO"), None);
    }

    #[test]
    fn test_display() {
        let mut coll = CommodityCollection::default();
        let eur = create_currency(&mut coll, "EUR");
        assert_eq!(
            coll.get(eur).unwrap().display(&dec!(0.238)),
            "0.24 EUR"
        );
        assert_eq!(
            coll.get(eur).unwrap().display(&dec!(0.234)),
            "0.23 EUR"
        );
        assert_eq!(
            coll.get(eur).unwrap().display(&dec!(0.235)),
            "0.23 EUR"
        );
        assert_eq!( // round to nearest even
            coll.get(eur).unwrap().display(&dec!(0.245)),
            "0.24 EUR"
        );
        assert_eq!(
            coll.get(eur).unwrap().display(&dec!(1.00)),
            "1.00 EUR"
        );
        assert_eq!(
            coll.get(eur).unwrap().display(&dec!(1)),
            "1.00 EUR"
        );
    }

}
