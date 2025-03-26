/// Fine-grained properties for accounts. Thanks to these flags, we can make
/// various computations.
use crate::account_categories::AccountCategory;
use case_insensitive_hashmap::CaseInsensitiveHashMap;
use std::{cell::RefCell, rc::Rc};

pub struct AccountKindCollection {
    kinds: CaseInsensitiveHashMap<AccountKind>,
}

impl Default for AccountKindCollection {
    fn default() -> Self {
        let mut d = Self {
            kinds: CaseInsensitiveHashMap::new(),
        };

        let all_default = vec![
            AccountKind::new(
                "Passive Income",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_passive_income(true),
            AccountKind::new(
                "Work Income",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_work_income(true),
            AccountKind::new(
                "Income",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            ),
            AccountKind::new(
                "Unrealized gain",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_unrealized(true),
            AccountKind::new(
                "Expense",
                "Increase",
                "Decrease",
                AccountCategory::EXPENSE,
            ),
            AccountKind::new(
                "Income tax",
                "Increase",
                "Decrease",
                AccountCategory::EXPENSE,
            )
            .set_is_income_tax(true),
            AccountKind::new(
                "Other tax",
                "Increase",
                "Decrease",
                AccountCategory::EXPENSE,
            )
            .set_is_misc_tax(true),
            AccountKind::new(
                "Liability",
                "Deposit",
                "Paiement",
                AccountCategory::LIABILITY,
            )
            .set_is_networth(true),
            AccountKind::new(
                "Equity",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            ),
            AccountKind::new(
                "Checking",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true),
            AccountKind::new(
                "Savings",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true),
            AccountKind::new("Stock", "Add", "Remove", AccountCategory::EQUITY)
                .set_is_networth(true)
                .set_is_trading(true)
                .set_is_stock(true),
            AccountKind::new(
                "Investment",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true)
            .set_is_trading(true),
            AccountKind::new(
                "Asset",
                "Increase",
                "Decrease",
                AccountCategory::ASSET,
            )
            .set_is_networth(true),
            AccountKind::new(
                "Non-liquid Investment",
                "Deposit",
                "Paiement",
                AccountCategory::ASSET,
            )
            .set_is_networth(true)
            .set_is_trading(true),
        ];

        for k in all_default {
            d.kinds.insert(k.get_name(), k);
        }
        d
    }
}

impl AccountKindCollection {
    /// Lookup an account that matches "Equity"
    pub fn get_equity(&self) -> AccountKind {
        self.kinds
            .values()
            .find(|k| k.is_equity())
            .expect("No account kind found for Equity")
            .clone()
    }

    /// Lookup account kind by name.
    /// This is case-insensitive.
    pub fn lookup(&self, name: &str) -> Option<&AccountKind> {
        self.kinds.get(name)
    }
}

#[derive(Debug)]
struct AccountKindDetails {
    // The name, used for display purposes only
    name: String,

    // credit / increase / ...
    _name_when_positive: String,

    // debit / decrease / ...
    _name_when_negative: String,

    category: AccountCategory,

    //-------------------------
    // Expenses and income

    // Whether this is an income category resulting from work activities, which
    // would disappear if you stopped working. This includes salary,
    // unemployment,...
    is_work_income: bool,

    // Whether this is an income category not resulting from work activities,
    // like dividends, rents,...
    is_passive_income: bool,

    // Whether this is a potential income or expense, i.e. the amount might
    // change later. This includes stock price changes, real-estate until you
    // actually sell, and so on. This is the result of your assets' value
    // changing over time.
    // When this is False, some money was actually transferred from/to one of
    // your accounts.
    is_unrealized: bool,

    //---------------------------
    // Networth
    /// True for all accounts used to compute the networth.
    /// It should be False for categories in general.
    is_networth: bool,

    //---------------------------
    // Investments

    // Whether the account should be displayed in the Investment and Performance
    // views.
    is_trading: bool,

    // An account used to trade one security
    is_stock: bool,

    //------------------------------
    // Taxes

    // Whether this category is part of your income taxes. This is used in the
    // metrics view to compute the actual tax rate.
    is_income_tax: bool,

    // Whether this should count as taxes, other than income taxes
    is_misc_tax: bool,
}

impl AccountKindDetails {
    pub fn new(
        name: &str,
        name_when_positive: &str,
        name_when_negative: &str,
        category: AccountCategory,
    ) -> Self {
        AccountKindDetails {
            name: name.into(),
            _name_when_positive: name_when_positive.into(),
            _name_when_negative: name_when_negative.into(),
            category,
            is_work_income: false,
            is_passive_income: false,
            is_unrealized: false,
            is_networth: false,
            is_trading: false,
            is_stock: false,
            is_income_tax: false,
            is_misc_tax: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AccountKind(Rc<RefCell<AccountKindDetails>>);

impl AccountKind {
    pub fn new(
        name: &str,
        name_when_positive: &str,
        name_when_negative: &str,
        category: AccountCategory,
    ) -> Self {
        AccountKind(Rc::new(RefCell::new(AccountKindDetails::new(
            name,
            name_when_positive,
            name_when_negative,
            category,
        ))))
    }
    pub fn set_is_work_income(self, is_work_income: bool) -> Self {
        self.0.borrow_mut().is_work_income = is_work_income;
        self
    }
    pub fn set_is_passive_income(self, is_passive_income: bool) -> Self {
        self.0.borrow_mut().is_passive_income = is_passive_income;
        self
    }
    pub fn set_is_unrealized(self, is_unrealized: bool) -> Self {
        self.0.borrow_mut().is_unrealized = is_unrealized;
        self
    }
    pub fn set_is_networth(self, is_networth: bool) -> Self {
        self.0.borrow_mut().is_networth = is_networth;
        self
    }
    pub fn set_is_trading(self, is_trading: bool) -> Self {
        self.0.borrow_mut().is_trading = is_trading;
        self
    }
    pub fn set_is_stock(self, is_stock: bool) -> Self {
        self.0.borrow_mut().is_stock = is_stock;
        self
    }
    pub fn set_is_income_tax(self, is_income_tax: bool) -> Self {
        self.0.borrow_mut().is_income_tax = is_income_tax;
        self
    }
    pub fn set_is_misc_tax(self, is_misc_tax: bool) -> Self {
        self.0.borrow_mut().is_misc_tax = is_misc_tax;
        self
    }

    pub fn is_expense(&self) -> bool {
        matches!(self.0.borrow().category, AccountCategory::EXPENSE)
    }

    pub fn is_income(&self) -> bool {
        matches!(self.0.borrow().category, AccountCategory::INCOME)
    }

    /// True if this kind works for Equity (e.g. reconciliation, initial
    /// balance,...)
    pub fn is_equity(&self) -> bool {
        matches!(self.0.borrow().category, AccountCategory::EQUITY)
            && !self.is_networth()
    }

    pub fn get_name(&self) -> String {
        self.0.borrow().name.clone()
    }

    pub fn cmp_name(&self, right: &AccountKind) -> std::cmp::Ordering {
        self.0.borrow().name.cmp(&right.0.borrow().name)
    }

    pub fn is_unrealized(&self) -> bool {
        self.0.borrow().is_unrealized
    }

    pub fn is_networth(&self) -> bool {
        self.0.borrow().is_networth
    }

    pub fn is_income_tax(&self) -> bool {
        self.0.borrow().is_income_tax
    }

    pub fn is_liquid(&self) -> bool {
        matches!(self.0.borrow().category, AccountCategory::EQUITY)
        && self.is_networth()
    }

    pub fn is_passive_income(&self) -> bool {
        self.0.borrow().is_passive_income
    }
}

impl PartialEq for AccountKind {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }
}

impl Eq for AccountKind {}
