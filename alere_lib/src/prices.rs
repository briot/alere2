use crate::commodities::CommodityId;
use crate::price_sources::PriceSourceId;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;

#[derive(Debug)]
pub struct Price {
    pub origin: CommodityId,
    pub target: CommodityId,
    timestamp: DateTime<Local>,
    pub price: Decimal,
    source: PriceSourceId,
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
            source,
        }
    }
}
