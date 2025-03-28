use crate::commodities::Commodity;
use crate::price_sources::PriceSourceFrom;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Default)]
pub struct PriceCollection {
    pub(crate) prices: HashMap<(Commodity, Commodity), Vec<Price>>,
}

impl PriceCollection {
    /// Register a new historical price.
    /// Prices are kept sorted so we can quickly look them up later.
    pub fn add(
        &mut self,
        origin: &Commodity,
        target: &Commodity,
        price: Price,
    ) {
        let p = self
            .prices
            .entry((origin.clone(), target.clone()))
            .or_default();
        let pos = match p.binary_search_by(|pr| pr.older_than(&price)) {
            Ok(pos) | Err(pos) => pos,
        };
        p.insert(pos, price);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Price {
    pub timestamp: DateTime<Local>,
    pub price: Decimal,
    _source: PriceSourceFrom,
}

impl Price {
    /// Create a new price
    pub fn new(
        timestamp: DateTime<Local>,
        price: Decimal,
        source: PriceSourceFrom,
    ) -> Self {
        Price {
            timestamp,
            price,
            _source: source,
        }
    }

    /// Compare two prices chronologically.
    /// We do not implement std::cmd::PartialOrd since it seems like the latter
    /// should compare actual prices.
    pub fn older_than(&self, price: &Price) -> std::cmp::Ordering {
        self.timestamp.cmp(&price.timestamp)
    }

    pub fn older_than_ts(&self, ts: &DateTime<Local>) -> std::cmp::Ordering {
        self.timestamp.cmp(ts)
    }

    pub fn more_recent_than_ts(
        &self,
        ts: &DateTime<Local>,
    ) -> std::cmp::Ordering {
        self.timestamp.cmp(ts).reverse()
    }

    /// Invert the price
    pub fn invert(&self) -> Price {
        Price {
            timestamp: self.timestamp,
            price: Decimal::ONE / self.price,
            _source: self._source,
        }
    }
}
