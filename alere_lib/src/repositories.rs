use crate::account_categories::AccountCategory;
use crate::account_kinds::{AccountKind, AccountKindCollection, AccountKindId};
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

pub struct Repository {
    institutions: HashMap<InstitutionId, Institution>,
    accounts: AccountCollection,
    account_kinds: AccountKindCollection,
    commodities: CommodityCollection,
    payees: HashMap<PayeeId, Payee>,
    price_sources: HashMap<PriceSourceId, PriceSource>,
    prices: PriceCollection,
    transactions: Vec<TransactionRc>,
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
            prices: PriceCollection::default(),
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
    // Re-arrange internal data structure for faster queries.  For instance
    // ensures that things are sorted by dates when appropriate.
    pub fn postprocess(&mut self) {
        self.prices.postprocess();
        self.accounts.postprocess();

        // ??? We should sort transactions, but they have no timestamps.  In
        // fact, what counts is sorting the splits themselves, when we compute
        // an account's balance at some point in time, for instance.
    }

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
    pub fn get_account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.get(id)
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

    /// Show the balance for each account
    pub fn balance(
        &self,
        as_of: &[DateTime<Local>],
    ) -> HashMap<AccountId, Vec<MultiValue>> {
        self.accounts
            .iter_accounts()
            .filter(|(_, acc)| {
                !acc.closed
                    && self.account_kinds.get(acc.kind).unwrap().is_networth
            })
            .map(|(acc_id, acc)| {
                let mut acc_balance = vec![MultiValue::default(); as_of.len()];

                //  ??? Could we use fold() here, though we are applying in
                //  place.
                acc.iter_transactions()
                    .flat_map(|tx| tx.iter_splits())
                    .filter(|s| s.account == acc_id)
                    .for_each(|s| {
                        for (idx, ts) in as_of.iter().enumerate() {
                            if s.post_ts <= *ts {
                                acc_balance[idx].apply(&s.original_value);
                            }
                        }
                    });
                (acc_id, acc_balance)
            })
            .collect()
    }
}

pub struct MarketPrices<'a> {
    cache: HashMap<CommodityId, Option<Price>>,
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
        match self.to_commodity {
            None => *value,
            Some(c) if c == value.commodity => *value,
            Some(c) => {
                let m =
                    self.cache.entry(value.commodity).or_insert_with(|| {
                        self.repo.prices.price_as_of(
                            value.commodity,
                            c,
                            self.repo.commodities.list_currencies(),
                            as_of,
                        )
                    });
                match m {
                    None => *value,
                    Some(m) => Value::new(m.price * value.value, c),
                }
            }
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

    pub fn get_prices(
        &mut self,
        value: &MultiValue,
        as_of: &DateTime<Local>,
    ) -> Vec<Value> {
        match self.to_commodity {
            None => vec![],
            Some(c) => value
                .iter()
                .filter(|v| v.commodity != c)
                .map(|v| {
                    self.convert_value(
                        &Value::new(Decimal::ONE, v.commodity),
                        as_of,
                    )
                })
                .collect(),
        }
    }
}
