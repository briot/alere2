use crate::account_kinds::AccountKindCollection;
use crate::accounts::{
    Account, AccountCollection, AccountId, AccountNameDepth,
};
use crate::commodities::{CommodityCollection, CommodityId};
use crate::formatters::Formatter;
use crate::institutions::{Institution, InstitutionId};
use crate::market_prices::MarketPrices;
use crate::multi_values::{MultiValue, Operation, Value};
use crate::payees::{Payee, PayeeId};
use crate::price_sources::{PriceSource, PriceSourceId};
use crate::prices::{Price, PriceCollection};
use crate::transactions::TransactionRc;
use std::collections::HashMap;

#[derive(Default)]
pub struct Repository {
    institutions: HashMap<InstitutionId, Institution>,
    accounts: AccountCollection,
    pub(crate) account_kinds: AccountKindCollection,
    pub commodities: CommodityCollection,
    payees: HashMap<PayeeId, Payee>,
    price_sources: HashMap<PriceSourceId, PriceSource>,
    pub(crate) prices: PriceCollection,
    pub(crate) transactions: Vec<TransactionRc>,
    pub format: Formatter,
}

impl Repository {
    /// Re-arrange internal data structure for faster queries.  For instance
    /// ensures that things are sorted by dates when appropriate.
    pub fn postprocess(&mut self) {
        self.prices.postprocess();
        self.accounts.postprocess();

        for tr in &self.transactions {
            if !tr.is_balanced() {
                println!("Transaction not balanced: {:?}", tr);
            }
        }
    }

    pub fn add_institution(&mut self, id: InstitutionId, inst: Institution) {
        self.institutions.insert(id, inst);
    }
    pub fn get_institution_mut(
        &mut self,
        id: &InstitutionId,
    ) -> Option<&mut Institution> {
        self.institutions.get_mut(id)
    }

    /// Return the institution to which an account belongs.  If the account
    /// itself doesn't specify this information, look in the parent account.
    pub fn get_account_institution(
        &self,
        acc: &Account,
    ) -> Option<&Institution> {
        let mut inst_id = acc.get_institution_id();
        let mut current = acc;
        while inst_id.is_none() {
            match current.get_parent_id() {
                None => {
                    break;
                }
                Some(p) => {
                    current = self.accounts.get(p).unwrap();
                    inst_id = current.get_institution_id();
                }
            }
        }
        inst_id.and_then(|inst| self.institutions.get(&inst))
    }

    pub fn add_account(&mut self, account: Account) -> AccountId {
        self.accounts.add(account)
    }
    pub fn get_account_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.accounts.get_mut(id)
    }
    pub fn get_account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.get(id)
    }
    pub fn iter_accounts(&self) -> impl Iterator<Item = (AccountId, &Account)> {
        self.accounts.iter_accounts()
    }

    /// Return the parent accounts of acc (not including acc itself).  The last
    /// element returned is the toplevel account, like Asset.
    pub fn iter_parent_accounts<'a>(
        &'a self,
        acc: &'a Account,
    ) -> impl Iterator<Item = &'a Account> {
        ParentAccountIter::new(self, acc).skip(1)
    }

    pub fn get_account_name(
        &self,
        acc: &Account,
        kind: AccountNameDepth,
    ) -> String {
        self.accounts.name(acc, kind)
    }

    pub fn add_price_source(&mut self, id: PriceSourceId, source: PriceSource) {
        self.price_sources.insert(id, source);
    }

    /// Returns the display precision for a given commodity.
    pub fn get_display_precision(&self, id: &CommodityId) -> u8 {
        self.commodities.get(*id).unwrap().display_precision
    }

    pub fn add_payee(&mut self, id: PayeeId, payee: Payee) {
        self.payees.insert(id, payee);
    }

    pub fn add_price(
        &mut self,
        origin: CommodityId,
        target: CommodityId,
        price: Price,
    ) {
        self.prices.add(origin, target, price);
    }

    pub fn add_transaction(&mut self, tx: &TransactionRc) {
        self.transactions.push(tx.clone());

        for s in tx.iter_splits() {
            // Add the transaction to each account it applies to
            self.accounts
                .get_mut(s.account)
                .unwrap()
                .add_transaction(tx);

            // Register prices from transactions
            match &s.operation {
                Operation::BuyAmount { qty, amount } => {
                    self.add_price(
                        amount.commodity,
                        qty.commodity,
                        Price::new(
                            s.post_ts,
                            qty.amount / amount.amount,
                            PriceSourceId::Transaction,
                        ),
                    );
                }
                Operation::BuyPrice { qty, price } => {
                    self.add_price(
                        price.commodity,
                        qty.commodity,
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
    }

    pub fn display_multi_value(&self, value: &MultiValue) -> String {
        let mut into = String::new();
        value.display(&mut into, &self.format, &self.commodities);
        into
    }
    pub fn display_value(&self, value: &Value) -> String {
        self.format.display_from_commodity(
            value.amount,
            self.commodities.get(value.commodity).unwrap(),
        )
    }

    pub fn market_prices(
        &self,
        to_commodity: Option<CommodityId>,
    ) -> MarketPrices {
        MarketPrices::new(
            &self.prices,
            self.commodities.list_currencies(),
            to_commodity,
        )
    }
}

pub struct ParentAccountIter<'a> {
    current: Option<&'a Account>,
    repo: &'a Repository,
}
impl<'a> ParentAccountIter<'a> {
    /// An iterator that return current and all its parent accounts
    pub fn new(repo: &'a Repository, current: &'a Account) -> Self {
        Self {
            repo,
            current: Some(current),
        }
    }
}
impl<'a> Iterator for ParentAccountIter<'a> {
    type Item = &'a Account;
    fn next(&mut self) -> Option<Self::Item> {
        let p = self.current;
        if let Some(c) = self.current {
            self.current =
                c.get_parent_id().and_then(|pid| self.repo.get_account(pid));
        }
        p
    }
}
