use crate::account_categories::AccountCategory;
use crate::account_kinds::{AccountKind, AccountKindCollection, AccountKindId};
use crate::accounts::{Account, AccountCollection, AccountId, AccountNameKind};
use crate::commodities::{Commodity, CommodityCollection, CommodityId};
use crate::institutions::{Institution, InstitutionId};
use crate::multi_values::{MultiValue, Value};
use crate::payees::{Payee, PayeeId};
use crate::price_sources::{PriceSource, PriceSourceId};
use crate::prices::Price;
use crate::transactions::{Quantity, Transaction};
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct Repository {
    institutions: HashMap<InstitutionId, Institution>,
    accounts: AccountCollection,
    account_kinds: AccountKindCollection,
    commodities: CommodityCollection,
    payees: HashMap<PayeeId, Payee>,
    price_sources: HashMap<PriceSourceId, PriceSource>,
    prices: Vec<Price>,
    transactions: Vec<Transaction>,
}

impl Default for Repository {
    fn default() -> Self {
        let mut repo = Repository {
            institutions: Default::default(),
            accounts: Default::default(),
            account_kinds: Default::default(),
            commodities: Default::default(),
            payees: Default::default(),
            price_sources: Default::default(),
            prices: Default::default(),
            transactions: Default::default(),
        };
        repo.add_account_kind(
            AccountKind::new(
                "Passive Income",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_passive_income(true),
        );
        repo.add_account_kind(
            AccountKind::new(
                "Work Income",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_work_income(true),
        );
        repo.add_account_kind(AccountKind::new(
            "Income",
            "Expense",
            "Income",
            AccountCategory::INCOME,
        ));
        repo.add_account_kind(
            AccountKind::new(
                "Unrealized gain",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_unrealized(true),
        );
        repo.add_account_kind(AccountKind::new(
            "Expense",
            "Increase",
            "Decrease",
            AccountCategory::EXPENSE,
        ));
        repo.add_account_kind(
            AccountKind::new(
                "Income tax",
                "Increase",
                "Decrease",
                AccountCategory::EXPENSE,
            )
            .set_is_income_tax(true),
        );
        repo.add_account_kind(
            AccountKind::new(
                "Other tax",
                "Increase",
                "Decrease",
                AccountCategory::EXPENSE,
            )
            .set_is_misc_tax(true),
        );
        repo.add_account_kind(
            AccountKind::new(
                "Liability",
                "Deposit",
                "Paiement",
                AccountCategory::LIABILITY,
            )
            .set_is_networth(true),
        );
        repo.add_account_kind(AccountKind::new(
            "Equity",
            "Deposit",
            "Paiement",
            AccountCategory::EQUITY,
        ));
        repo.add_account_kind(
            AccountKind::new(
                "Checking",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true),
        );
        repo.add_account_kind(
            AccountKind::new(
                "Savings",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true),
        );
        repo.add_account_kind(
            AccountKind::new("Stock", "Add", "Remove", AccountCategory::EQUITY)
                .set_is_networth(true)
                .set_is_trading(true)
                .set_is_stock(true),
        );
        repo.add_account_kind(
            AccountKind::new(
                "Investment",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true)
            .set_is_trading(true),
        );
        repo.add_account_kind(
            AccountKind::new(
                "Asset",
                "Increase",
                "Decrease",
                AccountCategory::ASSET,
            )
            .set_is_networth(true),
        );
        repo.add_account_kind(
            AccountKind::new(
                "Non-liquid Investment",
                "Deposit",
                "Paiement",
                AccountCategory::ASSET,
            )
            .set_is_networth(true)
            .set_is_trading(true),
        );
        repo
    }
}

impl Repository {
    pub fn add_account_kind(&mut self, kind: AccountKind) -> AccountKindId {
        self.account_kinds.add(kind)
    }

    pub fn get_account_kinds(&self) -> &AccountKindCollection {
        &self.account_kinds
    }

    pub fn add_institution(&mut self, id: InstitutionId, inst: Institution) {
        self.institutions.insert(id, inst);
    }

    pub fn add_account(&mut self, account: Account) -> AccountId {
        self.accounts.add(account)
    }
    pub fn get_account_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.accounts.get_mut(id)
    }
    pub fn get_account_name(
        &self,
        id: AccountId,
        kind: AccountNameKind,
    ) -> String {
        self.accounts.name(id, kind)
    }

    pub fn add_price_source(&mut self, id: PriceSourceId, source: PriceSource) {
        self.price_sources.insert(id, source);
    }

    pub fn add_commodity(&mut self, comm: Commodity) -> CommodityId {
        self.commodities.add(comm)
    }
    pub fn find_commodity(&self, name: &str) -> Option<CommodityId> {
        self.commodities.find(name)
    }

    /// Returns the display precision for a given commodity.
    pub fn get_precision(&self, id: &CommodityId) -> u8 {
        self.commodities.get(*id).unwrap().precision
    }

    pub fn add_payee(&mut self, id: PayeeId, payee: Payee) {
        self.payees.insert(id, payee);
    }

    pub fn add_price(&mut self, price: Price) {
        self.prices.push(price);
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    pub fn display_multi_value(&self, value: &MultiValue) -> String {
        value.display(&self.commodities)
    }

    pub fn market_prices(&self, to_commodity: Option<CommodityId>) -> MarketPrices {
        MarketPrices::new(self, to_commodity)
    }

    /// Show the balance for each account, either converted to the given
    /// commodity, using today's market prices, or in the original commodity.
    pub fn balance(&self) -> HashMap<AccountId, MultiValue> {
        let mut bal: HashMap<AccountId, MultiValue> = HashMap::new();
        for t in &self.transactions {
            for s in &t.splits {
                let acc = self.accounts.get(s.account).unwrap();
                if acc.closed
                    || !self.account_kinds.get(acc.kind).unwrap().is_networth
                {
                    continue;
                }

                match s.original_value {
                    Quantity::Credit(value) => {
                        bal.entry(s.account)
                            .and_modify(|v| *v += value)
                            .or_insert_with(|| MultiValue::from_value(value));
                    }
                    Quantity::Buy(shares) => {
                        bal.entry(s.account)
                            .and_modify(|v| *v += shares)
                            .or_insert_with(|| MultiValue::from_value(shares));
                    }
                    _ => {}
                };
            }
        }
        bal
    }
}

pub struct MarketPrices<'a> {
    cache: HashMap<CommodityId, Decimal>,
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
    ) -> Value {
        match self.to_commodity {
            None => *value,
            Some(c) if c == value.commodity => *value,
            Some(c) => {
                let m = self.cache
                    .entry(value.commodity)
                    .or_insert_with(|| {
                        println!("MANU search for xrate {:?}", self.to_commodity);
                        self.repo.prices.iter()
                            .rev()
                            .filter(|p|
                                (p.target == c && p.origin == value.commodity)
                                || (p.origin == c && p.target == value.commodity))
                            .nth(0)
                            .map(|p| {
                                if p.target == c {
                                    p.price
                                } else {
                                    Decimal::ONE / p.price
                                }
                            })
                            .unwrap_or(Decimal::ZERO)
                     });
                Value::new(*m * value.value, c)
            }
        }
    }

    pub fn convert_multi_value(
        &mut self,
        value: &MultiValue,
    ) -> MultiValue {
        let mut result = MultiValue::default();
        for v in value.iter() {
            result += self.convert_value(&v);
        }
        result
    }

}
