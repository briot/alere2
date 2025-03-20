use crate::account_kinds::AccountKind;
use crate::accounts::{Account, AccountId};
use crate::commodities::{Commodity, CommodityCollection};
use crate::institutions::Institution;
use crate::market_prices::MarketPrices;
use crate::multi_values::Operation;
use crate::payees::{Payee, PayeeId};
use crate::price_sources::{PriceSource, PriceSourceId};
use crate::prices::{Price, PriceCollection};
use crate::transactions::TransactionRc;
use case_insensitive_hashmap::CaseInsensitiveHashMap;
use itertools::min;
use std::collections::HashMap;

pub struct Repository {
    institutions: Vec<Institution>,
    accounts: Vec<Account>,
    account_kinds: CaseInsensitiveHashMap<AccountKind>,
    pub commodities: CommodityCollection,
    payees: HashMap<PayeeId, Payee>,
    price_sources: HashMap<PriceSourceId, PriceSource>,
    pub(crate) prices: PriceCollection,
    pub(crate) transactions: Vec<TransactionRc>,
}

impl Default for Repository {
    fn default() -> Self {
        let mut s = Self {
            institutions: Default::default(),
            accounts: Default::default(),
            account_kinds: CaseInsensitiveHashMap::new(),
            commodities: Default::default(),
            payees: Default::default(),
            price_sources: Default::default(),
            prices: Default::default(),
            transactions: Default::default(),
        };

        for k in AccountKind::all_default() {
            s.account_kinds.insert(k.get_name(), k);
        }

        s
    }
}

impl Repository {
    /// Re-arrange internal data structure for faster queries.  For instance
    /// ensures that things are sorted by dates when appropriate.
    pub fn postprocess(&mut self) {
        self.prices.postprocess();

        self.transactions.sort_by_cached_key(|tx| {
            min(tx.iter_splits().map(|s| s.post_ts)).unwrap()
        });

        for tr in &self.transactions {
            if !tr.is_balanced() {
                println!("Transaction not balanced: {:?}", tr);
            }
        }
    }

    /// Lookup an account that matches "Equity"
    pub fn get_equity_kind(&self) -> AccountKind {
        self.account_kinds
            .values()
            .find(|k| k.is_equity())
            .expect("No account kind found for Equity")
            .clone()
    }

    /// Lookup account kind by name.
    /// This is case-insensitive.
    pub fn lookup_kind(&self, name: &str) -> Option<&AccountKind> {
        self.account_kinds.get(name)
    }

    pub fn add_institution(&mut self, inst: Institution) {
        self.institutions.push(inst);
    }

    /// Register a new account.  This automatically sets the id
    pub fn add_account(&mut self, mut account: Account) -> Account {
        account.set_id(
            self.accounts
                .iter()
                .map(Account::get_id)
                .max()
                .unwrap_or(AccountId::default())
                .inc(),
        );
        self.accounts.push(account.clone());
        account
    }
    pub fn iter_accounts(&self) -> impl Iterator<Item = Account> + '_ {
        self.accounts.iter().cloned()
    }

    /// Return the parent accounts of acc (not including acc itself).  The last
    /// element returned is the toplevel account, like Asset.
    pub fn iter_parent_accounts(
        &self,
        acc: &Account,
    ) -> impl Iterator<Item = Account> + '_ {
        ParentAccountIter::new(acc.clone())
    }

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

    pub fn add_transaction(&mut self, tx: &TransactionRc) {
        self.transactions.push(tx.clone());

        for s in tx.iter_splits() {
            // Add the transaction to each account it applies to
            s.account.add_transaction(tx);

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

pub struct ParentAccountIter {
    current: Option<Account>,
}
impl ParentAccountIter {
    /// An iterator that return current and all its parent accounts
    pub fn new(current: Account) -> Self {
        Self {
            current: Some(current),
        }
    }
}
impl Iterator for ParentAccountIter {
    type Item = Account;
    fn next(&mut self) -> Option<Self::Item> {
        let p = match &self.current {
            None => None,
            Some(c) => c.get_parent().clone(),
        };
        self.current = p;
        self.current.clone()
    }
}
