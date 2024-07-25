use crate::commodities::CommodityId;
use crate::price_sources::PriceSourceId;
use bisection::bisect_right_by;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Default)]
pub struct PriceCollection {
    prices: HashMap<(CommodityId, CommodityId), Vec<Price>>,
}

impl PriceCollection {
    /// Register a new historical price

    pub fn add(
        &mut self,
        origin: CommodityId,
        target: CommodityId,
        price: Price,
    ) {
        self.prices
            .entry((origin, target))
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

    /// Find the current price at the given timestamp, using a variety of
    /// sources for the price.
    /// - historical prices, applying conversions in both directions
    /// - transactions, comparing value and original_value
    /// - indirect exchange rates by going via a turnkey currency

    pub fn price_as_of(
        &self,
        from: CommodityId,
        to: CommodityId,
        turnkeys: &[CommodityId],
        as_of: &DateTime<Local>,
    ) -> Option<Price> {
        let mut candidates: Vec<Price> = vec![];

        if let Some(p) = self.internal_get_price(from, to, as_of) {
            candidates.push(p.clone());
        }
        if let Some(p) = self.internal_get_price(to, from, as_of) {
            candidates.push(p.invert());
        }

        for turnkey in turnkeys {
            let p1 = self.price_as_of(from, *turnkey, &[], as_of);
            let p2 = self.price_as_of(*turnkey, to, &[], as_of);

            match (p1, p2) {
                (None, _) | (_, None) => {},
                (Some(p1), Some(p2)) => {
                    candidates.push(Price {
                        timestamp: std::cmp::min(p1.timestamp, p2.timestamp),
                        price: p1.price * p2.price,
                        _source: PriceSourceId::Turnkey,
                    });
                }
            }
        }

        candidates
            .iter()
            .max_by(|p1, p2| p1.older_than(p2))
            .cloned()
    }

    fn internal_get_price(
        &self,
        from: CommodityId,
        to: CommodityId,
        as_of: &DateTime<Local>,
    ) -> Option<&Price> {
        let pr = self.prices.get(&(from, to));
        if let Some(pr) = pr {
            // `p` is the first index for which the timestamp is greater or
            // equal to `as_of`.
            let p = bisect_right_by(pr, |p| p.more_recent_than_ts(as_of));
            if p != 0 {
                return Some(&pr[p - 1]);
            }
        }
        None
    }

}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Price {
    timestamp: DateTime<Local>,
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

    pub fn more_recent_than_ts(&self, ts: &DateTime<Local>) -> std::cmp::Ordering {
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

#[cfg(test)]
mod test {
    use crate::prices::{PriceCollection, Price};
    use crate::commodities::CommodityId;
    use crate::price_sources::PriceSourceId;
    use chrono::{Days, Local, TimeZone};
    use rust_decimal_macros::dec;

    #[test]
    fn test_price_as_of() {
        let mut prices = PriceCollection::default();
        let origin = CommodityId(1);
        let target = CommodityId(2);
        let turnkey = CommodityId(3);
        let target2 = CommodityId(4);
        let t1 = Local.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(
            prices.price_as_of(origin, target, &[turnkey], &t1),
            None,
        );

        prices.add(
            origin,
            target,
            Price::new(t1, dec!(0.2), PriceSourceId::Transaction));
        assert_eq!(  //  before first price
            prices.price_as_of(origin, target, &[turnkey], &(t1 - Days::new(1))),
            None,  
        );
        assert_eq!(  //  exactly first price
            prices.price_as_of(origin, target, &[turnkey], &t1),
            Some(Price::new(t1, dec!(0.2), PriceSourceId::Transaction)), 
        );
        assert_eq!(  //  invert xrate
            prices.price_as_of(target, origin, &[turnkey], &t1),
            Some(Price::new(t1, dec!(5), PriceSourceId::Transaction)), 
        );
        assert_eq!(  //  after last price
            prices.price_as_of(origin, target, &[turnkey], &(t1 + Days::new(3))),
            Some(Price::new(t1, dec!(0.2), PriceSourceId::Transaction)), 
        );

        // Second price is in reverse order
        prices.add(
            target,
            origin,
            Price::new(t1 + Days::new(2), dec!(4), PriceSourceId::Transaction));
        assert_eq!(   //  before first price
            prices.price_as_of(origin, target, &[turnkey], &(t1 - Days::new(1))),
            None,
        );
        assert_eq!(   //  between two days, use earlier price
            prices.price_as_of(origin, target, &[turnkey], &(t1 + Days::new(1))),
            Some(Price::new(t1, dec!(0.2), PriceSourceId::Transaction)), 
        );
        assert_eq!(   //  on second day
            prices.price_as_of(origin, target, &[turnkey], &(t1 + Days::new(2))),
            Some(Price::new(t1 + Days::new(2), dec!(0.25), PriceSourceId::Transaction)), 
        );
        assert_eq!(   //  after last price
            prices.price_as_of(origin, target, &[turnkey], &(t1 + Days::new(3))),
            Some(Price::new(t1 + Days::new(2), dec!(0.25), PriceSourceId::Transaction)), 
        );

        // Third price not in chronological order
        prices.add(
            origin,
            target,
            Price::new(t1 - Days::new(1), dec!(0.6), PriceSourceId::Transaction));
        prices.postprocess();  // need sorting
        assert_eq!(
            prices.price_as_of(origin, target, &[turnkey], &t1),
            Some(Price::new(t1, dec!(0.2), PriceSourceId::Transaction)), 
        );

        // Test turnkeys
        prices.add(
            origin,
            turnkey,
            Price::new(t1, dec!(0.7), PriceSourceId::Transaction));
        prices.add(
            target2,
            turnkey,
            Price::new(t1, dec!(0.8), PriceSourceId::Transaction));
        assert_eq!(
            prices.price_as_of(origin, target2, &[turnkey], &t1),
            Some(Price::new(t1, dec!(0.7) / dec!(0.8), PriceSourceId::Turnkey)), 
        );
        assert_eq!(
            prices.price_as_of(target2, origin, &[turnkey], &t1),
            Some(Price::new(t1, dec!(0.8) / dec!(0.7), PriceSourceId::Turnkey)), 
        );
    }

}
