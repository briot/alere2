use crate::price_sources::PriceSourceFrom;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CommodityId(u32);

#[derive(Clone, Debug)]
pub struct Commodity(Rc<RefCell<CommodityDetails>>);

impl Commodity {
    /// Return a persistent, unique id
    pub fn get_id(&self) -> CommodityId {
        self.0.borrow().id
    }

    pub fn is_currency(&self) -> bool {
        self.0.borrow().is_currency
    }

    /// Returns the display precision for a given commodity.
    pub fn get_display_precision(&self) -> u8 {
        self.0.borrow().display_precision
    }

    pub fn get_name(&self) -> Ref<'_, String> {
        Ref::map(self.0.borrow(), |d| &d.name)
    }

    pub fn get_symbol(&self) -> Ref<'_, String> {
        Ref::map(self.0.borrow(), |d| &d.symbol)
    }

    pub fn symbol_after(&self) -> bool {
        self.0.borrow().symbol_after
    }

    pub fn set_isin(&mut self, isin: &str) {
        self.0.borrow_mut().isin = Some(isin.to_string());
    }

    pub fn matches(&self, name: &str) -> bool {
        self.0.borrow().name == name
    }
}

impl PartialEq for Commodity {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }
}

impl Eq for Commodity {}

impl std::hash::Hash for Commodity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get_id().hash(state);
    }
}

#[derive(Default)]
pub struct CommodityCollection {
    commodities: Vec<Commodity>,
    currencies: Vec<Commodity>, // duplicates some of those in commodities
}

impl CommodityCollection {
    pub fn add(
        &mut self,
        name: &str,
        symbol: &str,
        symbol_after: bool,
        is_currency: bool,
        quote_symbol: Option<&str>,
        display_precision: u8,
    ) -> Commodity {
        let c = Commodity(Rc::new(RefCell::new(CommodityDetails {
            id: CommodityId(
                self.commodities
                    .iter()
                    .map(|c| c.0.borrow().id.0)
                    .max()
                    .unwrap_or(0)
                    + 1,
            ),
            name: name.into(),
            display_precision,
            symbol: symbol.trim().to_string(),
            symbol_after,
            is_currency,
            _quote_symbol: quote_symbol.map(str::to_string),
            _quote_source: None,
            _quote_currency: None,
            isin: None,
        })));

        if is_currency {
            self.currencies.push(c.clone());
        }
        self.commodities.push(c.clone());
        c
    }

    #[cfg(test)]
    pub fn add_dummy(&mut self, name: &str, is_currency: bool) -> Commodity {
        self.add(name, name, false, is_currency, None, 2)
    }

    pub fn list_currencies(&self) -> &[Commodity] {
        &self.currencies
    }

    pub fn iter_commodities(&self) -> impl Iterator<Item = &Commodity> {
        self.commodities.iter()
    }

    pub fn find(&self, name: &str) -> Option<Commodity> {
        self.commodities.iter().find(|c| c.matches(name)).cloned()
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
struct CommodityDetails {
    /// Unique, persistent id.  Used for hashing
    id: CommodityId,

    /// Name as displayed in selection boxes in the GUI.  For instance, it
    /// could be "Euro", "Apple Inc.", ...
    name: String,

    // Symbol to display the commodity. For instance, it could be the
    // euro sign, or "AAPL", and whether to display before or after the value.
    symbol: String,
    symbol_after: bool,

    /// What kind of commodity this is.
    is_currency: bool,

    /// ISIN number
    isin: Option<String>,

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
    _quote_symbol: Option<String>,
    _quote_source: Option<PriceSourceFrom>,
    _quote_currency: Option<Commodity>,

    /// Number of digits in the fractional part
    display_precision: u8,
}

#[cfg(test)]
pub mod test {
    use crate::commodities::{Commodity, CommodityCollection};

    pub fn create_currency(
        coll: &mut CommodityCollection,
        name: &str,
        precision: u8,
        after: bool,
    ) -> Commodity {
        coll.add(name, name, after, true, None, precision)
    }

    pub fn create_security(
        coll: &mut CommodityCollection,
        name: &str,
    ) -> Commodity {
        coll.add(name, name, true, false, None, 2)
    }

    #[test]
    fn test_commodity() {
        let mut coll = CommodityCollection::default();
        let eur = create_currency(&mut coll, "EUR", 2, true);
        let aapl = create_security(&mut coll, "AAPL");
        assert_eq!(coll.list_currencies(), &[eur.clone()]);
        assert_eq!(coll.find("EUR"), Some(eur));
        assert_eq!(coll.find("AAPL"), Some(aapl));
        assert_eq!(coll.find("FOO"), None);
    }
}
