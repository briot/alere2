/// Fine-grained properties for accounts. Thanks to these flags, we can make
/// various computations.
use crate::account_categories::AccountCategory;

pub struct AccountKindCollection(pub Vec<AccountKind>);

impl AccountKindCollection {
    pub fn add(&mut self, account: AccountKind) -> AccountKindId {
        self.0.push(account);
        AccountKindId(self.0.len() as u32)
    }

    pub fn get(&self, id: AccountKindId) -> Option<&AccountKind> {
        self.0.get(id.0 as usize - 1)
    }
}

impl Default for AccountKindCollection {
    fn default() -> Self {
        let mut a = Self(Vec::new());

        a.add(
            AccountKind::new(
                "Passive Income",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_passive_income(true),
        );
        a.add(
            AccountKind::new(
                "Work Income",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_work_income(true),
        );
        a.add(AccountKind::new(
            "Income",
            "Expense",
            "Income",
            AccountCategory::INCOME,
        ));
        a.add(
            AccountKind::new(
                "Unrealized gain",
                "Expense",
                "Income",
                AccountCategory::INCOME,
            )
            .set_is_unrealized(true),
        );
        a.add(AccountKind::new(
            "Expense",
            "Increase",
            "Decrease",
            AccountCategory::EXPENSE,
        ));
        a.add(
            AccountKind::new(
                "Income tax",
                "Increase",
                "Decrease",
                AccountCategory::EXPENSE,
            )
            .set_is_income_tax(true),
        );
        a.add(
            AccountKind::new(
                "Other tax",
                "Increase",
                "Decrease",
                AccountCategory::EXPENSE,
            )
            .set_is_misc_tax(true),
        );
        a.add(
            AccountKind::new(
                "Liability",
                "Deposit",
                "Paiement",
                AccountCategory::LIABILITY,
            )
            .set_is_networth(true),
        );
        a.add(AccountKind::new(
            "Equity",
            "Deposit",
            "Paiement",
            AccountCategory::EQUITY,
        ));
        a.add(
            AccountKind::new(
                "Checking",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true),
        );
        a.add(
            AccountKind::new(
                "Savings",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true),
        );
        a.add(
            AccountKind::new("Stock", "Add", "Remove", AccountCategory::EQUITY)
                .set_is_networth(true)
                .set_is_trading(true)
                .set_is_stock(true),
        );
        a.add(
            AccountKind::new(
                "Investment",
                "Deposit",
                "Paiement",
                AccountCategory::EQUITY,
            )
            .set_is_networth(true)
            .set_is_trading(true),
        );
        a.add(
            AccountKind::new(
                "Asset",
                "Increase",
                "Decrease",
                AccountCategory::ASSET,
            )
            .set_is_networth(true),
        );
        a.add(
            AccountKind::new(
                "Non-liquid Investment",
                "Deposit",
                "Paiement",
                AccountCategory::ASSET,
            )
            .set_is_networth(true)
            .set_is_trading(true),
        );
        a
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct AccountKindId(pub u32);

#[derive(Debug)]
pub struct AccountKind {
    // The name, used for display purposes only
    pub name: String,

    // credit / increase / ...
    pub name_when_positive: String,

    // debit / decrease / ...
    pub name_when_negative: String,

    pub category: AccountCategory,

    //-------------------------
    // Expenses and income

    // Whether this is an income category resulting from work activities, which
    // would disappear if you stopped working. This includes salary,
    // unemployment,...
    pub is_work_income: bool,

    // Whether this is an income category not resulting from work activities,
    // like dividends, rents,...
    pub is_passive_income: bool,

    // Whether this is a potential income or expense, i.e. the amount might
    // change later. This includes stock price changes, real-estate until you
    // actually sell, and so on. This is the result of your assets' value
    // changing over time.
    // When this is False, some money was actually transferred from/to one of
    // your accounts.
    pub is_unrealized: bool,

    //---------------------------
    // Networth
    /// True for all accounts used to compute the networth.
    /// It should be False for categories in general.
    pub is_networth: bool,

    //---------------------------
    // Investments

    // Whether the account should be displayed in the Investment and Performance
    // views.
    pub is_trading: bool,

    // An account used to trade one security
    pub is_stock: bool,

    //------------------------------
    // Taxes

    // Whether this category is part of your income taxes. This is used in the
    // metrics view to compute the actual tax rate.
    pub is_income_tax: bool,

    // Whether this should count as taxes, other than income taxes
    pub is_misc_tax: bool,
}

impl AccountKind {
    pub fn new(
        name: &str,
        name_when_positive: &str,
        name_when_negative: &str,
        category: AccountCategory,
    ) -> Self {
        AccountKind {
            name: name.into(),
            name_when_positive: name_when_positive.into(),
            name_when_negative: name_when_negative.into(),
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

    pub fn set_is_work_income(mut self, is_work_income: bool) -> Self {
        self.is_work_income = is_work_income;
        self
    }
    pub fn set_is_passive_income(mut self, is_passive_income: bool) -> Self {
        self.is_passive_income = is_passive_income;
        self
    }
    pub fn set_is_unrealized(mut self, is_unrealized: bool) -> Self {
        self.is_unrealized = is_unrealized;
        self
    }
    pub fn set_is_networth(mut self, is_networth: bool) -> Self {
        self.is_networth = is_networth;
        self
    }
    pub fn set_is_trading(mut self, is_trading: bool) -> Self {
        self.is_trading = is_trading;
        self
    }
    pub fn set_is_stock(mut self, is_stock: bool) -> Self {
        self.is_stock = is_stock;
        self
    }
    pub fn set_is_income_tax(mut self, is_income_tax: bool) -> Self {
        self.is_income_tax = is_income_tax;
        self
    }
    pub fn set_is_misc_tax(mut self, is_misc_tax: bool) -> Self {
        self.is_misc_tax = is_misc_tax;
        self
    }

    pub fn is_expense(&self) -> bool {
        matches!(self.category, AccountCategory::EXPENSE)
    }
    pub fn is_income(&self) -> bool {
        matches!(self.category, AccountCategory::INCOME)
    }
}
