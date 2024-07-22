use crate::commodities::CommodityId;
use crate::price_sources::PriceSourceId;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use std::cmp::Ordering;
use std::collections::HashMap;

// We have various sources for prices:
// - historical prices, applying conversions in both directions
// - transactions, comparing value and original_value
// - indirect exchange rates by going via a turnkey currency

#[derive(Default)]
pub struct PriceCollection {
    prices: Vec<Price>,
    latest: HashMap<(CommodityId, CommodityId), (DateTime<Local>, Decimal)>,
}

impl PriceCollection {
    pub fn add(&mut self, price: Price) {
        self.latest
            .entry((price.origin, price.target))
            .and_modify(|(ts, v)| {
                if *ts < price.timestamp {
                    *v = price.price;
                }
            })
            .or_insert((price.timestamp, price.price));
        self.prices.push(price);
    }

    pub fn latest_price(
        &self,
        from: CommodityId,
        to: CommodityId,
        currencies: &[CommodityId],
    ) -> Option<Decimal> {
        let mut candidates = vec![
            self.latest.get(&(from, to)).cloned(),
            self.latest.get(&(to, from)).cloned(),
        ];

        currencies
            .iter()
            .map(|turnkey| {
                let p1 = self.latest.get(&(from, *turnkey));
                let p2 = self.latest.get(&(*turnkey, to));
                match (p1, p2) {
                    (None, _) | (_, None) => None,
                    (Some((t1, v1)), Some((t2, v2))) => {
                        Some((std::cmp::min(*t1, *t2), v1 * v2))
                    }
                }
            })
            .for_each(|c| candidates.push(c));

        candidates
            .iter()
            .filter_map(|p| *p)
            .max_by(|p1, p2| {
                if p1.0 < p2.0 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            })
            .map(|v| v.1)
    }
}

#[derive(Debug)]
pub struct Price {
    pub origin: CommodityId,
    pub target: CommodityId,
    timestamp: DateTime<Local>,
    pub price: Decimal,
    _source: PriceSourceId,
}

impl Price {
    pub fn new(
        origin: CommodityId,
        target: CommodityId,
        timestamp: DateTime<Local>,
        price: Decimal,
        source: PriceSourceId,
    ) -> Self {
        Price {
            origin,
            target,
            timestamp,
            price,
            _source: source,
        }
    }
}
