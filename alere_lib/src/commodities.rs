use crate::price_sources::PriceSourceId;

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

    pub fn iter_commodities(
        &self,
    ) -> impl Iterator<Item = (CommodityId, &Commodity)> {
        self.commodities
            .iter()
            .enumerate()
            .map(|(idx, c)| (CommodityId(idx as u16 + 1), c))
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

pub struct Commodity {
    /// Name as displayed in selection boxes in the GUI.  For instance, it
    /// could be "Euro", "Apple Inc.", ...
    pub name: String,

    // Symbol to display the commodity. For instance, it could be the
    // euro sign, or "AAPL", and whether to display before or after the value.
    pub(crate) symbol: String,
    pub(crate) symbol_after: bool,

    /// What kind of commodity this is.
    pub is_currency: bool,

    /// ISIN number
    pub isin: Option<String>,

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
    pub(crate) display_precision: u8,
}

impl Commodity {
    pub fn new(
        name: &str,
        symbol: &str,
        symbol_after: bool,
        is_currency: bool,
        quote_symbol: Option<&str>,
        display_precision: u8,
    ) -> Self {
        Commodity {
            name: name.into(),
            display_precision,
            symbol: symbol.trim().to_string(),
            symbol_after,
            is_currency,
            quote_symbol: quote_symbol.map(str::to_string),
            quote_source: None,
            quote_currency: None,
            isin: None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::commodities::{Commodity, CommodityCollection, CommodityId};

    pub fn create_currency(
        coll: &mut CommodityCollection,
        name: &str,
        precision: u8,
        after: bool,
    ) -> CommodityId {
        let commodity =
            Commodity::new(name, name, after, true, None, precision);
        coll.add(commodity)
    }

    pub fn create_security(
        coll: &mut CommodityCollection,
        name: &str,
    ) -> CommodityId {
        let commodity = Commodity::new(name, name, true, false, None, 2);
        coll.add(commodity)
    }

    #[test]
    fn test_commodity() {
        let mut coll = CommodityCollection::default();
        let eur = create_currency(&mut coll, "EUR", 2, true);
        let aapl = create_security(&mut coll, "AAPL");
        assert_eq!(coll.list_currencies(), &[eur]);
        assert_eq!(coll.find("EUR"), Some(eur));
        assert_eq!(coll.find("AAPL"), Some(aapl));
        assert_eq!(coll.find("FOO"), None);
    }
}
