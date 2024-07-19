use crate::price_sources::PriceSourceId;
use rust_decimal::Decimal;

#[derive(Default)]
pub struct CommodityCollection(Vec<Commodity>);

impl CommodityCollection {
    pub fn add(&mut self, commodity: Commodity) -> CommodityId {
        self.0.push(commodity);
        CommodityId(self.0.len() as u16)
    }

    pub fn get_mut(&mut self, id: CommodityId) -> Option<&mut Commodity> {
        self.0.get_mut(id.0 as usize - 1)
    }

    pub fn get(&self, id: CommodityId) -> Option<&Commodity> {
        self.0.get(id.0 as usize - 1)
    }

    pub fn find(&self, name: &str) -> Option<CommodityId> {
        self.0.iter().enumerate().find(|(_, c)| c.name == name)
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
    pub precision: u8,
}

impl Commodity {
    pub fn new(
        name: &str,
        symbol_before: &str,
        symbol_after: &str,
        is_currency: bool,
        quote_symbol: Option<&str>,
        precision: u8,
    ) -> Self {
        Commodity {
            name: name.into(),
            symbol_before: symbol_before.trim().to_string(),
            symbol_after: symbol_after.trim().into(),
            is_currency,
            quote_symbol: quote_symbol.map(str::to_string),
            quote_source: None,
            quote_currency: None,
            precision,
        }
    }

    //  Display a given value for a commodity
    pub fn display(&self, value: &Decimal) -> String {
        format!(
            "{}{}{}{}{}",
            self.symbol_before,
            if self.symbol_before.is_empty() {
                ""
            } else {
                " "
            },
            value,
            if self.symbol_after.is_empty() {
                ""
            } else {
                " "
            },
            self.symbol_after
        )
    }
}
