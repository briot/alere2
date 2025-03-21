use crate::{
    account_kinds::AccountKindCollection,
    accounts::AccountCollection,
    commodities::{Commodity, CommodityCollection},
    institutions::InstitutionCollection,
    market_prices::MarketPrices,
    multi_values::Operation,
    payees::PayeeCollection,
    price_sources::{PriceSourceCollection, PriceSourceFrom},
    prices::{Price, PriceCollection},
    transactions::{Transaction, TransactionCollection},
};
use anyhow::Result;

#[derive(Default)]
pub struct Repository {
    pub(crate) institutions: InstitutionCollection,
    pub(crate) accounts: AccountCollection,
    pub(crate) account_kinds: AccountKindCollection,
    pub commodities: CommodityCollection,
    pub(crate) payees: PayeeCollection,
    pub(crate) price_sources: PriceSourceCollection,
    pub(crate) prices: PriceCollection,
    pub(crate) transactions: TransactionCollection,
}

impl Repository {
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<()> {
        for s in tx.splits().iter() {
            // Register prices from transactions
            match &s.operation {
                Operation::BuyAmount { qty, amount } => {
                    self.prices.add(
                        &amount.commodity,
                        &qty.commodity,
                        Price::new(
                            s.post_ts,
                            qty.amount / amount.amount,
                            PriceSourceFrom::Transaction,
                        ),
                    );
                }
                Operation::BuyPrice { qty, price } => {
                    self.prices.add(
                        &price.commodity,
                        &qty.commodity,
                        Price::new(
                            s.post_ts,
                            price.amount,
                            PriceSourceFrom::Transaction,
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
