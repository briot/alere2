use crate::accounts::AccountId;
use crate::commodities::CommodityId;
use crate::multi_values::MultiValue;
use crate::repositories::{MarketPrices, Repository};
use crate::trees::Tree;
use crate::utils::is_all_same;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;

//--------------------------------------------------------------
// Settings
//--------------------------------------------------------------

pub struct Settings {
    // Do not show rows if the value is zero
    pub hide_zero: bool,

    // Do not show rows if the value or market_value hasn't changed between
    // all timestamps.  If there is a single timestamp, rows are always
    // displayed.
    pub hide_all_same: bool,

    // Display a tree of accounts
    pub tree: bool,

    // If true, parents' values will also including all their children
    pub subtotals: bool,

    // Currency for market values
    pub commodity: Option<CommodityId>,
}

//--------------------------------------------------------------
// Balance
//--------------------------------------------------------------

/// The balance of an account, computed as of a specific timestamp
#[derive(Clone, Default, PartialEq)]
pub struct Balance {
    value: MultiValue,
    market_value: MultiValue,
}

impl Balance {
    /// Compute the market value as the given timestamp, by converting the
    /// value.  For instance, the account's value might be 8 shares of AAPL,
    /// and this is converts to 1840 EUR.
    fn compute_market(
        &mut self,
        prices: &mut MarketPrices,
        as_of: &DateTime<Local>,
    ) {
        if !self.value.is_zero() {
            let mv = prices.convert_multi_value(&self.value, as_of);
            self.market_value = mv;
        } else {
            self.market_value = MultiValue::default();
        }
    }

    /// True if the value is zero.
    /// We do not check the market_value, which will be zero also in that case.
    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    /// Compute the price used to convert from value to market_value.
    /// If we have multiple commodities, this returns nothing.
    pub fn get_price(&self) -> Option<Decimal> {
        let mut p = None;
        self.value.iter().enumerate().for_each(|(idx, v)| {
            if idx > 0 || v.value.is_zero() {
                p = None;
            } else {
                self.market_value.iter().for_each(|mv| {
                    let x = mv.value / v.value;
                    p = if x == Decimal::ONE { None } else { Some(x) };
                });
            }
        });
        p
    }
}

impl core::ops::AddAssign<&Balance> for Balance {
    fn add_assign(&mut self, rhs: &Balance) {
        self.value += &rhs.value;
        self.market_value += &rhs.market_value;
    }
}

impl core::ops::Sub<&Balance> for &Balance {
    type Output = Balance;

    fn sub(self, rhs: &Balance) -> Self::Output {
        Balance {
            value: &self.value - &rhs.value,
            market_value: &self.market_value - &rhs.market_value,
        }
    }
}

//--------------------------------------------------------------
// NetworthRow
//--------------------------------------------------------------

/// Represents one row of the networth: it is the balance of one account,
/// potentially computed for multiple timestamps.
#[derive(Clone)]
pub struct NetworthRow(Vec<Balance>);

impl NetworthRow {
    fn new(size: usize) -> Self {
        NetworthRow(vec![Balance::default(); size])
    }

    /// Whether the balance is zero for all the timestamps.
    /// This is in general used to filter out irrelevant rows.
    fn is_zero(&self) -> bool {
        self.0.iter().all(Balance::is_zero)
    }

    /// Whether the balance is the same for all timestamps.
    fn is_all_same(&self) -> bool {
        is_all_same(&self.0)
    }

    pub fn display_value(&self, repo: &Repository, idx: usize) -> String {
        repo.display_multi_value(&self.0[idx].value)
    }
    pub fn display_market_value(
        &self,
        repo: &Repository,
        idx: usize,
    ) -> String {
        repo.display_multi_value(&self.0[idx].market_value)
    }
    pub fn display_delta(&self, repo: &Repository, idx: usize) -> String {
        repo.display_multi_value(&(&self.0[idx + 1] - &self.0[idx]).value)
    }
    pub fn display_delta_to_last(
        &self,
        repo: &Repository,
        idx: usize,
    ) -> String {
        repo.display_multi_value(&(self.0.last().unwrap() - &self.0[idx]).value)
    }
    pub fn display_market_delta(
        &self,
        repo: &Repository,
        idx: usize,
    ) -> String {
        repo.display_multi_value(
            &(&self.0[idx + 1] - &self.0[idx]).market_value,
        )
    }
    pub fn display_market_delta_to_last(
        &self,
        repo: &Repository,
        idx: usize,
    ) -> String {
        repo.display_multi_value(
            &(self.0.last().unwrap() - &self.0[idx]).market_value,
        )
    }

    /// Show the price used to compute the market value of the idx-th column
    pub fn display_price(&self, idx: usize) -> String {
        let p = self.0[idx].get_price();
        match p {
            None => String::new(),
            Some(p) => p.to_string(),
        }
    }
}

impl core::ops::AddAssign<&NetworthRow> for NetworthRow {
    fn add_assign(&mut self, rhs: &NetworthRow) {
        self.0.iter_mut().zip(&rhs.0).for_each(|(v1, v2)| *v1 += v2);
    }
}

//--------------------------------------------------------------
// Networth
//--------------------------------------------------------------

/// A view that shows the value (as of any timestamp) of all user accounts.
/// This ignores all accounts that are not marked as "networth".
pub struct Networth {
    pub tree: Tree<AccountId, NetworthRow>,
    pub total: NetworthRow,
    pub settings: Settings,
    pub as_of: Vec<DateTime<Local>>,
}

impl Networth {
    /// Cumulate all operations, for all accounts, to get the current total.
    pub fn new(
        repo: &Repository,
        as_of: &[DateTime<Local>],
        settings: Settings,
    ) -> Self {
        let col_count = as_of.len();
        let mut market = repo.market_prices(settings.commodity);
        let mut result = Networth {
            settings,
            as_of: as_of.into(),
            tree: Tree::default(),
            total: NetworthRow::new(col_count),
        };

        repo.iter_accounts()
            .filter(move |(_, acc)| {
                !acc.closed
                    && repo
                        .get_account_kinds()
                        .get(acc.kind)
                        .unwrap()
                        .is_networth
            })
            .for_each(|(acc_id, acc)| {
                let parents = repo.get_account_parents(acc_id);
                let row = result.tree.try_get(&acc_id, &parents, |_| {
                    NetworthRow::new(col_count)
                });

                //  ??? Could we use fold() here, though we are applying in
                //  place.
                acc.iter_splits(acc_id).for_each(|s| {
                    for (idx, ts) in as_of.iter().enumerate() {
                        if s.post_ts <= *ts {
                            row.0[idx].value.apply(&s.original_value);
                        }
                    }
                });

                for (idx, v) in row.0.iter_mut().enumerate() {
                    v.compute_market(&mut market, &as_of[idx]);
                    result.total.0[idx] += v;
                }
            });

        // Filter out rows.  This needs to be done after we have inserted them
        // all above, including the parents, since the values might not be known
        // till that point.
        result.tree.retain(|node| {
            node.has_children()   // Always keep parent nodes with children
            || (
                (!result.settings.hide_zero || !node.data.data.is_zero())
                && (!result.settings.hide_all_same
                    || !node.data.data.is_all_same()))
        });

        if result.settings.subtotals {
            result.tree.traverse_mut(
                |node| {
                    let mut tmp = NetworthRow::new(col_count);
                    node.iter_children().for_each(|child| {
                        tmp += &child.data.data;
                    });
                    node.data.data += &tmp;
                },
                false,
            );
        }

        result.tree.sort(|d1, d2| {
            repo.get_account(d1.key)
                .unwrap()
                .name
                .cmp(&repo.get_account(d2.key).unwrap().name)
        });
        result
    }
}
