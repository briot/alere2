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
use chrono::{DateTime, Local};

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
    #[must_use]
    pub fn transactions(&self) -> &TransactionCollection {
        &self.transactions
    }

    #[must_use]
    pub fn prices(&self) -> &PriceCollection {
        &self.prices
    }

    #[must_use]
    pub fn accounts(&self) -> &AccountCollection {
        &self.accounts
    }

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
                Operation::Credit(_)
                | Operation::AddShares { .. }
                | Operation::Reinvest { .. }
                | Operation::Dividend
                | Operation::Split { .. } => {}
            }
        }
        self.transactions.add(tx)?;
        Ok(())
    }

    #[must_use]
    pub fn market_prices(
        &self,
        to_commodity: Option<Commodity>,
    ) -> MarketPrices<'_> {
        MarketPrices::new(
            &self.prices,
            self.commodities.list_currencies(),
            to_commodity,
        )
    }

    pub fn add_price(
        &mut self,
        origin: &Commodity,
        target: &Commodity,
        price: Price,
    ) {
        self.prices.add(origin, target, price);
    }

    #[must_use]
    #[allow(clippy::mutable_key_type)]
    pub fn compute_commodity_balances(
        &self,
    ) -> std::collections::HashMap<Commodity, rust_decimal::Decimal> {
        #[allow(clippy::mutable_key_type)]
        let mut balances = std::collections::HashMap::new();
        for account in self.accounts.iter() {
            account.for_each_split(|split| {
                let mut mv = crate::multi_values::MultiValue::default();
                mv.apply(&split.operation);
                for value in mv.iter() {
                    *balances
                        .entry(value.commodity.clone())
                        .or_insert(rust_decimal::Decimal::ZERO) += value.amount;
                }
            });
        }
        balances
    }

    /// Find earliest transaction date
    #[must_use]
    pub fn earliest_transaction_date(&self) -> Option<DateTime<Local>> {
        let mut earliest = None;
        for tx in self.transactions().iter() {
            for split in tx.splits().iter() {
                if earliest.is_none() || split.post_ts < earliest.unwrap() {
                    earliest = Some(split.post_ts);
                }
            }
        }
        earliest
    }
}
