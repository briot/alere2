use crate::commodities::Commodity;
use crate::price_sources::PriceSourceId;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Default)]
pub struct PriceCollection {
    pub(crate) prices: HashMap<(Commodity, Commodity), Vec<Price>>,
}

impl PriceCollection {
    /// Register a new historical price
    pub fn add(
        &mut self,
        origin: &Commodity,
        target: &Commodity,
        price: Price,
    ) {
        self.prices
            .entry((origin.clone(), target.clone()))
            .or_default()
            // ??? Should we use bisection::insort_left_by
            .push(price);
    }

    /// Pre-process all prices to ensure that prices are sorted in a way that
    /// we can quickly look them up later on.
    pub fn postprocess(&mut self) {
        for (_, v) in self.prices.iter_mut() {
            v.sort_by(Price::older_than);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Price {
    pub timestamp: DateTime<Local>,
    pub price: Decimal,
    _source: PriceSourceId,
}

impl Price {
    /// Create a new price
    pub fn new(
        timestamp: DateTime<Local>,
        price: Decimal,
        source: PriceSourceId,
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
