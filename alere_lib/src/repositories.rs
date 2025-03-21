use crate::{
    account_kinds::AccountKindCollection,
    accounts::AccountCollection,
    commodities::{Commodity, CommodityCollection},
    institutions::InstitutionCollection,
    market_prices::MarketPrices,
    multi_values::Operation,
    payees::PayeeCollection,
    price_sources::{PriceSource, PriceSourceId},
    prices::{Price, PriceCollection},
    transactions::{Transaction, TransactionCollection},
};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Default)]
pub struct Repository {
    pub(crate) institutions: InstitutionCollection,
    pub(crate) accounts: AccountCollection,
    pub(crate) account_kinds: AccountKindCollection,
    pub commodities: CommodityCollection,
    pub(crate) payees: PayeeCollection,
    price_sources: HashMap<PriceSourceId, PriceSource>,
    pub(crate) prices: PriceCollection,
    pub(crate) transactions: TransactionCollection,
}

impl Repository {
    pub fn add_price_source(&mut self, id: PriceSourceId, source: PriceSource) {
        self.price_sources.insert(id, source);
    }

    pub fn add_price(
        &mut self,
        origin: &Commodity,
        target: &Commodity,
        price: Price,
    ) {
        self.prices.add(origin, target, price);
    }

    pub fn add_transaction(&mut self, tx: Transaction) -> Result<()> {
        for s in tx.splits().iter() {
            // Register prices from transactions
            match &s.operation {
                Operation::BuyAmount { qty, amount } => {
                    self.add_price(
                        &amount.commodity,
                        &qty.commodity,
                        Price::new(
                            s.post_ts,
                            qty.amount / amount.amount,
                            PriceSourceId::Transaction,
                        ),
                    );
                }
                Operation::BuyPrice { qty, price } => {
                    self.add_price(
                        &price.commodity,
                        &qty.commodity,
                        Price::new(
                            s.post_ts,
                            price.amount,
                            PriceSourceId::Transaction,
                        ),
                    );
                }
                _ => {}
            }
        }
        self.transactions.add(tx)?;
        Ok(())
    }

    pub fn market_prices(
        &self,
        to_commodity: Option<Commodity>,
    ) -> MarketPrices {
        MarketPrices::new(
            &self.prices,
            self.commodities.list_currencies(),
            to_commodity,
        )
    }
}
