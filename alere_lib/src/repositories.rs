use crate::account_kinds::AccountKindCollection;
use crate::accounts::AccountCollection;
use crate::commodities::{Commodity, CommodityCollection};
use crate::institutions::InstitutionCollection;
use crate::market_prices::MarketPrices;
use crate::multi_values::Operation;
use crate::payees::{Payee, PayeeId};
use crate::price_sources::{PriceSource, PriceSourceId};
use crate::prices::{Price, PriceCollection};
use crate::transactions::{Transaction, TransactionCollection};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Default)]
pub struct Repository {
    pub institutions: InstitutionCollection,
    pub accounts: AccountCollection,
    pub account_kinds: AccountKindCollection,
    pub commodities: CommodityCollection,
    payees: HashMap<PayeeId, Payee>,
    price_sources: HashMap<PriceSourceId, PriceSource>,
    pub(crate) prices: PriceCollection,
    pub(crate) transactions: TransactionCollection,
}

impl Repository {
    pub fn add_price_source(&mut self, id: PriceSourceId, source: PriceSource) {
        self.price_sources.insert(id, source);
    }

    pub fn add_payee(&mut self, id: PayeeId, payee: Payee) {
        self.payees.insert(id, payee);
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
