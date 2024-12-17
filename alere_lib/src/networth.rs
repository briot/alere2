use crate::accounts::{Account, AccountId};
use crate::commodities::CommodityId;
use crate::market_prices::MarketPrices;
use crate::multi_values::MultiValue;
use crate::repositories::Repository;
use crate::times::{Intv, TimeInterval};
use crate::tree_keys::Key;
use crate::trees::Tree;
use crate::utils::is_all_same;
use anyhow::Result;
use chrono::{DateTime, Local};
use itertools::Itertools;
use rust_decimal::Decimal;

//--------------------------------------------------------------
// Settings
//--------------------------------------------------------------

#[derive(Copy, Clone)]
pub enum GroupBy {
    None,
    ParentAccount,
    AccountKind,
    Institution,
}
impl GroupBy {
    /// Whether output should reserve space for indentation
    pub fn need_indent(&self) -> bool {
        matches!(self, GroupBy::None)
    }
}

pub struct Settings {
    // Do not show rows if the value is zero
    pub hide_zero: bool,

    // Do not show rows if the value or market_value hasn't changed between
    // all timestamps.  If there is a single timestamp, rows are always
    // displayed.
    pub hide_all_same: bool,

    // Display a tree of accounts
    pub group_by: GroupBy,

    // If true, parents' values will also including all their children
    pub subtotals: bool,

    // Currency for market values
    pub commodity: Option<CommodityId>,

    // "boring" accounts are combined with their subaccount, unless no elide
    // is used.  Boring accounts have no balance of their own and just one
    // subaccount.
    // Instead of showing
    //     Asset
    //        MyBank
    //            MyAccount     $100
    //        OtherBank         $200
    // We now show
    //     Asset
    //        MyBank:MyAccount  $100
    //        OtherBank         $200
    // Only relevant for GroupBy::ParentAccount
    // A node is not collapsed if any operation applied to it directly.
    //
    pub elide_boring_accounts: bool,

    // What columns to display.  Each column aggregates all transaction within
    // a time interval.
    pub intervals: Vec<Intv>,
    // ??? Could have max_depth parameter
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
        self.market_value = prices.convert_multi_value(&self.value, as_of);
    }

    /// True if the value is zero.
    /// We do not check the market_value, which will be zero also in that case.
    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    /// Compute the price used to convert from value to market_value.
    /// If we have multiple commodities, this returns nothing.
    pub fn get_price(&self) -> Option<Decimal> {
        &self.market_value / &self.value
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

    /// Display value as percent of the total
    pub fn display_percent(&self, total: &Self, idx: usize) -> String {
        let percent = &self.0[idx].market_value / &total.0[idx].market_value;
        match percent {
            None => String::new(),
            Some(p) => format!("{:.1}%", p * Decimal::ONE_HUNDRED),
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
/// The result tree is unsorted.
pub struct Networth<'a> {
    pub tree: Tree<Key<'a>, NetworthRow>,
    pub total: NetworthRow,
    pub settings: Settings,
    pub intervals: Vec<TimeInterval>, //  Each column
}

impl<'a> Networth<'a> {
    /// Cumulate all operations, for all accounts, to get the current total.
    pub fn new<F: FnMut(&(AccountId, &Account)) -> bool>(
        repo: &'a Repository,
        settings: Settings,
        now: DateTime<Local>,
        account_filter: F,
    ) -> Result<Self> {
        let intervals = settings
            .intervals
            .iter()
            .map(|intv| intv.to_ranges(now))
            .flatten_ok() // itertools: preserve errors
            .collect::<Result<Vec<TimeInterval>>>()?;

        let col_count = intervals.len();

        // We keep one cache for market prices per entry in as_of, since
        // otherwise the current cache would keeping wiped out as we move from
        // one column to the next.

        let mut market = repo.market_prices(settings.commodity);
        let mut result = Networth {
            settings,
            intervals,
            tree: Tree::default(),
            total: NetworthRow::new(col_count),
        };

        repo.iter_accounts().filter(account_filter).for_each(
            |(acc_id, acc)| {
                let key = Key::Account(acc);
                let newcol = |_: &Key| NetworthRow::new(col_count);
                let row = match &result.settings.group_by {
                    GroupBy::None => {
                        result.tree.try_get(&key, std::iter::empty(), newcol)
                    }
                    GroupBy::ParentAccount => result.tree.try_get(
                        &key,
                        repo.iter_parent_accounts(acc).map(Key::Account),
                        newcol,
                    ),
                    GroupBy::AccountKind => result.tree.try_get(
                        &key,
                        std::iter::once(Key::AccountKind(
                            repo.account_kinds.get(acc.kind),
                        )),
                        newcol,
                    ),
                    GroupBy::Institution => result.tree.try_get(
                        &key,
                        std::iter::once(Key::Institution(
                            repo.get_account_institution(acc),
                        )),
                        newcol,
                    ),
                };

                //  ??? Could we use fold() here, though we are applying in
                //  place.
                acc.iter_splits(acc_id).for_each(|s| {
                    for (idx, intv) in result.intervals.iter().enumerate() {
                        if intv.intv.contains(&s.post_ts) {
                            row.0[idx].value.apply(&s.operation);
                        }
                    }
                });

                for (idx, v) in row.0.iter_mut().enumerate() {
                    v.compute_market(
                        &mut market,
                        // At end of interval (but this is open, so is not
                        // full accurate).
                        &result.intervals[idx]
                            .intv
                            .upper()
                            .expect("bounded interval"),
                    );
                    result.total.0[idx] += v;
                }
            },
        );

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

        if result.settings.elide_boring_accounts
            && matches!(result.settings.group_by, GroupBy::ParentAccount)
        {
            let _ = result.tree.traverse_mut(
                |node| {
                    if node.data.data.is_zero() {
                        node.collapse_if_one_child();
                    }
                    Ok(())
                },
                false,
            );
        }

        if result.settings.subtotals {
            let _ = result.tree.traverse_mut(
                |node| {
                    let mut tmp = NetworthRow::new(col_count);
                    node.iter_children().for_each(|child| {
                        tmp += &child.data.data;
                    });
                    node.data.data += &tmp;
                    Ok(())
                },
                false,
            );
        }

        Ok(result)
    }
}
