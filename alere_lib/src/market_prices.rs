use crate::commodities::Commodity;
use crate::multi_values::{MultiValue, Value};
use crate::price_sources::PriceSourceId;
use crate::prices::{Price, PriceCollection};
use bisection::bisect_right_by;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// A struct that can return the current market prices for a commodity, at any
/// point in time.
/// It is optimized to cache values, and efficiently optimize queries if you
/// do them in chronological order.
pub struct MarketPrices<'a> {
    cache: HashMap<(Commodity, Commodity), PairCacheLine>,
    known_prices: &'a PriceCollection,
    turnkey_currencies: &'a [Commodity],
    to_commodity: Option<Commodity>,
}

impl<'a> MarketPrices<'a> {
    /// Will compute market values into to_commodity, by using prices from
    /// the repository.
    /// If to_commodity is None, no conversion is made.
    pub fn new(
        known_prices: &'a PriceCollection,
        turnkey_currencies: &'a [Commodity],
        to_commodity: Option<Commodity>,
    ) -> Self {
        MarketPrices {
            known_prices,
            turnkey_currencies,
            to_commodity,
            cache: HashMap::new(),
        }
    }

    /// Convert each component of the multi-value to to_commodity, and sum
    /// the results.  We still return a Value, since we might be missing
    /// some exchange-rates, and could therefore left some of the components
    /// unconverted.
    pub fn convert_multi_value(
        &mut self,
        value: &MultiValue,
        as_of: &DateTime<Local>,
    ) -> MultiValue {
        let mut result = MultiValue::default();
        for pair in value.iter() {
            result += match self.get_price(&pair.commodity, as_of) {
                None => MultiValue::new(pair.amount, &pair.commodity),
                Some(p) => MultiValue::new(
                    p * pair.amount,
                    self.to_commodity.as_ref().unwrap(),
                ),
            };
        }
        result
    }

    pub fn convert_value(
        &mut self,
        value: &Value,
        as_of: &DateTime<Local>,
    ) -> MultiValue {
        match self.get_price(&value.commodity, as_of) {
            None => MultiValue::new(value.amount, &value.commodity),
            Some(p) => MultiValue::new(
                p * value.amount,
                self.to_commodity.as_ref().unwrap(),
            ),
        }
    }

    /// Return the price for the specified commodity.
    /// The prices are computed using various sources: either direct exchange
    /// rates (or reverse one, if we only knew that one); or perhaps going
    /// through a turnkey currency (like USD).
    pub fn get_price(
        &mut self,
        commodity: &Commodity,
        as_of: &DateTime<Local>,
    ) -> Option<Decimal> {
        match self.to_commodity.clone() {
            None => None,
            Some(c) if c == *commodity => Some(Decimal::ONE),
            Some(c) => {
                let mut result =
                    self.get_price_no_turnkey(commodity, &c, as_of);

                for turnkey in
                    self.turnkey_currencies.iter().filter(|curr| **curr != c)
                {
                    match self.get_price_no_turnkey(commodity, turnkey, as_of) {
                        None => {}
                        Some(p1) => match self
                            .get_price_no_turnkey(turnkey, &c, as_of)
                        {
                            None => {}
                            Some(p2) => {
                                keep_most_recent(
                                    &mut result,
                                    Price::new(
                                        std::cmp::min(
                                            p1.timestamp,
                                            p2.timestamp,
                                        ),
                                        p1.price * p2.price,
                                        PriceSourceId::Turnkey,
                                    ),
                                );
                            }
                        },
                    }
                }
                result.map(|m| m.price)
            }
        }
    }

    /// Compute prices by looking at exchange rate and reverse exchange rate,
    /// but not going through turnkey currencies.
    fn get_price_no_turnkey(
        &mut self,
        from: &Commodity,
        to: &Commodity,
        as_of: &DateTime<Local>,
    ) -> Option<Price> {
        let mut result: Option<Price> = self.lookup_price(from, to, as_of);
        if let Some(p) = self.lookup_price(to, from, as_of) {
            keep_most_recent(&mut result, p.invert());
        }
        result
    }

    /// Lookup a direct exchange rate, possibly reusing an existing cache.
    fn lookup_price(
        &mut self,
        from: &Commodity,
        to: &Commodity,
        as_of: &DateTime<Local>,
    ) -> Option<Price> {
        let key = (from.clone(), to.clone());

        let line = self.cache.get(&key);
        match line {
            // We have never looked up that pair before, so we basically
            // have to look everywhere
            None => {
                let newline = self.bisect(from, to, as_of, None);
                let found = newline.found.as_ref().map(|f| f.1.clone());
                self.cache.insert(key, newline);
                found
            }
            Some(line) => {
                // We have previously found something, can we just reuse it ?
                if line.request_ts != *as_of {
                    let newline = self.bisect(from, to, as_of, Some(line));
                    let found = newline.found.as_ref().map(|f| f.1.clone());
                    self.cache.insert(key, newline);
                    found
                } else {
                    line.found.as_ref().map(|f| f.1.clone())
                }
            }
        }
    }

    /// Look in the known prices for a direct exchange rate from FROM to TO
    /// (doesn't look at reverse exchange rates).
    /// It doesn't use the cache.
    fn bisect(
        &self,
        from: &Commodity,
        to: &Commodity,
        as_of: &DateTime<Local>,
        old: Option<&PairCacheLine>,
    ) -> PairCacheLine {
        match self.known_prices.prices.get(&(from.clone(), to.clone())) {
            // If there are no known prices, nothing to return
            None => PairCacheLine {
                request_ts: *as_of,
                found: None,
            },
            Some(prices) => {
                let (base_idx, all_prices) = match old {
                    // If we did a previous search, we can optimize the
                    // search by only looking at part of the array
                    // ??? Not the most efficient if the user is going by
                    // chronological order though.
                    Some(PairCacheLine {
                        request_ts: req,
                        found: Some(f),
                    }) => {
                        if req < as_of {
                            (f.0, &prices[f.0..])
                        } else {
                            (0, &prices[0..f.0])
                        }
                    }

                    // Do a full search if we did not find anything before
                    None
                    | Some(PairCacheLine {
                        request_ts: _,
                        found: None,
                    }) => (0_usize, prices.as_slice()),
                };

                let index = base_idx
                    + bisect_right_by(all_prices, |p| {
                        p.more_recent_than_ts(as_of)
                    });
                if index == 0 {
                    PairCacheLine {
                        request_ts: *as_of,
                        found: None,
                    }
                } else {
                    PairCacheLine {
                        request_ts: *as_of,
                        found: Some((index, prices[index - 1].clone())),
                    }
                }
            }
        }
    }
}

/// A cache line for one pair of commodities (e.g. (APPL, EUR)).  We have
/// previously looked it up for the given timestamp, and possibly found a value
/// at the given index.
/// When later the user does another query, we can either reuse exactly that
/// entry (if the request_ts is the same), or know whether we need to look in
/// prior or later entries.
struct PairCacheLine {
    request_ts: DateTime<Local>,
    found: Option<(usize, Price)>,
}

/// Keep the most recent of two prices
fn keep_most_recent(left: &mut Option<Price>, right: Price) {
    match left {
        None => *left = Some(right),
        Some(r) => {
            if matches!(r.older_than(&right), std::cmp::Ordering::Less) {
                *left = Some(right);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::commodities::CommodityCollection;
    use crate::market_prices::MarketPrices;
    use crate::price_sources::PriceSourceId;
    use crate::prices::{Price, PriceCollection};
    use chrono::{Days, Local, TimeZone};
    use rust_decimal_macros::dec;

    #[test]
    fn test_price_as_of() {
        let mut prices = PriceCollection::default();
        let mut coms = CommodityCollection::default();
        let origin = coms.add_dummy("origin", false);
        let target = coms.add_dummy("target", true);
        let turnkey = coms.add_dummy("turnkey", false);
        let turnkeys = [turnkey.clone()];
        let target2 = coms.add_dummy("target2", true);
        let t1 = Local.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();

        {
            let mut to_target =
                MarketPrices::new(&prices, &turnkeys, Some(target.clone()));
            assert_eq!(to_target.get_price(&origin, &t1), None);
        }

        prices.add(
            &origin,
            &target,
            Price::new(t1, dec!(0.2), PriceSourceId::Transaction),
        );

        {
            let mut to_target =
                MarketPrices::new(&prices, &turnkeys, Some(target.clone()));
            assert_eq!(
                //  before first price
                to_target.get_price(&origin, &(t1 - Days::new(1))),
                None,
            );
            assert_eq!(
                //  exactly first price
                to_target.get_price(&origin, &t1),
                Some(dec!(0.2)),
            );
            assert_eq!(
                //  after last price
                to_target.get_price(&origin, &(t1 + Days::new(3))),
                Some(dec!(0.2)),
            );
        }

        //  invert xrate
        {
            let mut to_origin =
                MarketPrices::new(&prices, &turnkeys, Some(origin.clone()));
            assert_eq!(to_origin.get_price(&target, &t1), Some(dec!(5)),);
        }

        // Second price is in reverse order
        prices.add(
            &target,
            &origin,
            Price::new(t1 + Days::new(2), dec!(4), PriceSourceId::Transaction),
        );
        {
            let mut to_target =
                MarketPrices::new(&prices, &turnkeys, Some(target.clone()));
            assert_eq!(
                //  before first price
                to_target.get_price(&origin, &(t1 - Days::new(1))),
                None,
            );
            assert_eq!(
                //  between two days, use earlier price
                to_target.get_price(&origin, &(t1 + Days::new(1))),
                Some(dec!(0.2)),
            );
            assert_eq!(
                //  on second day
                to_target.get_price(&origin, &(t1 + Days::new(2))),
                Some(dec!(0.25)),
            );
            assert_eq!(
                //  after last price
                to_target.get_price(&origin, &(t1 + Days::new(3))),
                Some(dec!(0.25)),
            );
        }

        // Third price not in chronological order
        prices.add(
            &origin,
            &target,
            Price::new(
                t1 - Days::new(1),
                dec!(0.6),
                PriceSourceId::Transaction,
            ),
        );

        {
            let mut to_target =
                MarketPrices::new(&prices, &turnkeys, Some(target.clone()));
            assert_eq!(to_target.get_price(&origin, &t1), Some(dec!(0.2)),);
        }

        // Test turnkeys
        prices.add(
            &origin,
            &turnkey,
            Price::new(t1, dec!(0.7), PriceSourceId::Transaction),
        );
        prices.add(
            &target2,
            &turnkey,
            Price::new(t1, dec!(0.8), PriceSourceId::Transaction),
        );
        {
            let mut to_target2 =
                MarketPrices::new(&prices, &turnkeys, Some(target2.clone()));
            let mut to_origin =
                MarketPrices::new(&prices, &turnkeys, Some(origin.clone()));
            assert_eq!(
                to_target2.get_price(&origin, &t1),
                Some(dec!(0.7) / dec!(0.8)),
            );
            assert_eq!(
                to_origin.get_price(&target2, &t1),
                Some(dec!(0.8) / dec!(0.7)),
            );
        }
    }
}
