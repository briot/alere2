use crate::account_kinds::AccountKindCollection;
use crate::accounts::{Account, AccountCollection, AccountId, AccountNameKind};
use crate::commodities::{Commodity, CommodityCollection, CommodityId};
use crate::institutions::{Institution, InstitutionId};
use crate::multi_values::{MultiValue, Operation, Value};
use crate::payees::{Payee, PayeeId};
use crate::price_sources::{PriceSource, PriceSourceId};
use crate::prices::{Price, PriceCollection};
use crate::transactions::TransactionRc;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Default)]
pub struct Repository {
    institutions: HashMap<InstitutionId, Institution>,
    accounts: AccountCollection,
    pub(crate) account_kinds: AccountKindCollection,
    commodities: CommodityCollection,
    payees: HashMap<PayeeId, Payee>,
    price_sources: HashMap<PriceSourceId, PriceSource>,
    prices: PriceCollection,
    transactions: Vec<TransactionRc>,
}

impl Repository {
    /// Re-arrange internal data structure for faster queries.  For instance
    /// ensures that things are sorted by dates when appropriate.
    pub fn postprocess(&mut self) {
        self.prices.postprocess();
        self.accounts.postprocess();

        // ??? We should sort transactions, but they have no timestamps.  In
        // fact, what counts is sorting the splits themselves, when we compute
        // an account's balance at some point in time, for instance.
    }

    pub fn add_institution(&mut self, id: InstitutionId, inst: Institution) {
        self.institutions.insert(id, inst);
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

    /// Return the parent accounts, starting with the direct parent.  The last
    /// element in the returned vec is therefore the toplevel account like
    /// Asset.
    pub fn get_account_parents_id(&self, id: AccountId) -> Vec<AccountId> {
        let mut parents = Vec::new();
        let mut p = id;
        while let Some(pid) = self.accounts.get(p).unwrap().get_parent_id() {
            parents.push(pid);
            p = pid;
        }
        parents
    }

    pub fn get_account_parents<'a, 'b>(
        &'a self,
        acc: &'b Account,
    ) -> Vec<&'a Account>
    where
        'b: 'a,
    {
        let mut parents = Vec::new();
        let mut p = acc;
        while let Some(pid) = p.get_parent_id() {
            p = self.accounts.get(pid).unwrap();
            parents.push(p);
        }
        parents
    }

    pub fn get_account_name(
        &self,
        acc: &Account,
        kind: AccountNameKind,
    ) -> String {
        self.accounts.name(acc, kind)
    }

    pub fn add_price_source(&mut self, id: PriceSourceId, source: PriceSource) {
        self.price_sources.insert(id, source);
    }

    pub fn add_commodity(&mut self, comm: Commodity) -> CommodityId {
        self.commodities.add(comm)
    }
    pub fn get_commodity(&self, id: CommodityId) -> Option<&Commodity> {
        self.commodities.get(id)
    }
    pub fn find_commodity(&self, name: &str) -> Option<CommodityId> {
        self.commodities.find(name)
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
            match (s.value, &s.original_value) {
                (Some(v), Operation::Buy(ov) | Operation::Credit(ov))
                    if v.commodity != ov.commodity =>
                {
                    // Register the price we paid
                    self.add_price(
                        ov.commodity,
                        v.commodity,
                        Price::new(
                            s.post_ts,
                            v.value / ov.value,
                            PriceSourceId::Transaction,
                        ),
                    );
                }
                _ => {}
            }
        }
    }

    pub fn display_multi_value(&self, value: &MultiValue) -> String {
        value.display(&self.commodities)
    }
    pub fn display_value(&self, value: &Value) -> String {
        value.display(&self.commodities)
    }

    pub fn market_prices(
        &self,
        to_commodity: Option<CommodityId>,
    ) -> MarketPrices {
        MarketPrices::new(self, to_commodity)
    }
}

pub struct MarketPrices<'a> {
    cache: HashMap<(CommodityId, DateTime<Local>), Option<Price>>,
    repo: &'a Repository,
    to_commodity: Option<CommodityId>,
}

impl<'a> MarketPrices<'a> {
    fn new(repo: &'a Repository, to_commodity: Option<CommodityId>) -> Self {
        MarketPrices {
            repo,
            to_commodity,
            cache: HashMap::new(),
        }
    }

    /// Return the current market price for commodity, given in to_commodity.
    /// Market acts as a cache.
    /// If to_commodity is None, no conversion is made.
    pub fn convert_value(
        &mut self,
        value: &Value,
        as_of: &DateTime<Local>,
    ) -> Value {
        let p = self.get_price(value.commodity, as_of);
        if p == Decimal::ONE {
            *value
        } else {
            Value::new(p * value.value, self.to_commodity.unwrap())
        }
    }

    pub fn convert_multi_value(
        &mut self,
        value: &MultiValue,
        as_of: &DateTime<Local>,
    ) -> MultiValue {
        let mut result = MultiValue::default();
        for v in value.iter() {
            result += self.convert_value(&v, as_of);
        }
        result
    }

    pub fn get_price(
        &mut self,
        commodity: CommodityId,
        as_of: &DateTime<Local>,
    ) -> Decimal {
        match self.to_commodity {
            None => Decimal::ONE,
            Some(c) if c == commodity => Decimal::ONE,
            Some(c) => {
                let m = self.cache.entry((commodity, *as_of)).or_insert_with(
                    || {
                        self.repo.prices.price_as_of(
                            commodity,
                            c,
                            self.repo.commodities.list_currencies(),
                            as_of,
                        )
                    },
                );
                match m {
                    None => Decimal::ONE,
                    Some(m) => m.price,
                }
            }
        }
    }
}
