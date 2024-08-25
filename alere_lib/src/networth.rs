use crate::accounts::{AccountId, AccountNameKind};
use crate::commodities::CommodityId;
use crate::multi_values::MultiValue;
use crate::repositories::{AccountBalance, Repository};
use crate::trees::Tree;
use crate::utils::is_all_same;
use chrono::{DateTime, Local};

pub struct Settings {
    // Which columns to show
    pub column_value: bool,
    pub column_market: bool,
    pub column_delta: bool,
    pub column_market_delta: bool,

    // Do not show rows if the value is zero
    pub hide_zero: bool,

    // Do not show rows if the value or market_value hasn't changed between
    // all timestamps.  If there is a single timestamp, rows are always
    // displayed.
    pub hide_all_same: bool,

    pub account_names: AccountNameKind,

    // Display a tree of accounts
    pub tree: bool,

    // If true, parents' values will also including all their children
    pub subtotals: bool,

    // Currency for market values
    pub commodity: Option<CommodityId>,
}

#[derive(Clone)]
pub struct NetworthRow {
    value: Vec<MultiValue>,
    market_value: Vec<MultiValue>,
}

impl NetworthRow {
    fn new(size: usize) -> Self {
        NetworthRow {
            value: vec![MultiValue::default(); size],
            market_value: vec![MultiValue::default(); size],
        }
    }

    pub fn display_value(&self, repo: &Repository, idx: usize) -> String {
        repo.display_multi_value(&self.value[idx])
    }
    pub fn display_market_value(
        &self,
        repo: &Repository,
        idx: usize,
    ) -> String {
        repo.display_multi_value(&self.market_value[idx])
    }
    pub fn display_delta(&self, repo: &Repository, idx: usize) -> String {
        repo.display_multi_value(&(&self.value[idx + 1] - &self.value[idx]))
    }
    pub fn display_market_delta(
        &self,
        repo: &Repository,
        idx: usize,
    ) -> String {
        repo.display_multi_value(
            &(&self.market_value[idx + 1] - &self.market_value[idx]),
        )
    }

    /// Add values to the already known values in self
    fn add_value(&mut self, values: &[MultiValue]) {
        self.value
            .iter_mut()
            .zip(values)
            .for_each(|(r, val)| *r += val);
    }
    fn add_market_value(&mut self, values: &[MultiValue]) {
        self.market_value
            .iter_mut()
            .zip(values)
            .for_each(|(r, val)| *r += val);
    }

    //    /// Merge two rows
    //    fn merge(&mut self, right: &NetworthRow) {
    //        for idx in 0..self.value.len() {
    //            self.value[idx] += &right.value[idx];
    //            self.market_value[idx] += &right.market_value[idx];
    //        }
    //    }
}

/// A view that shows the value (as of any timestamp) of all user accounts.
/// This ignores all accounts that are not marked as "networth".
pub struct Networth {
    pub tree: Tree<AccountId, NetworthRow>,
    pub total: NetworthRow,
    pub settings: Settings,
    pub as_of: Vec<DateTime<Local>>,
}

impl Networth {
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

        for AccountBalance(account, value) in repo.balance(as_of, |acc| {
            !acc.closed
                && repo.get_account_kinds().get(acc.kind).unwrap().is_networth
        }) {
            let parents = repo.get_account_parents(account);
            let row = result
                .tree
                .try_get(&account, &parents, |_| NetworthRow::new(col_count));
            row.add_value(&value);

            for (idx, v) in row.value.iter().enumerate() {
                if !v.is_zero() {
                    let mv = market.convert_multi_value(v, &as_of[idx]);
                    result.total.value[idx] += v;
                    result.total.market_value[idx] += &mv;
                    row.market_value[idx] = mv;
                }
            }
        }

        // Filter out rows.  This needs to be done after we have inserted them
        // all above, including the parents, since the values might not be known
        // till that point.
        result.tree.retain(|node| {
            node.has_children()   // Always keep parent nodes with children
            || (
                (!result.settings.hide_zero
                 || !node.data.data.value.iter().all(MultiValue::is_zero)
                 || !node.data.data.market_value.iter().all(MultiValue::is_zero))
                && (!result.settings.hide_all_same
                    || !is_all_same(&node.data.data.value)
                    || !is_all_same(&node.data.data.market_value)))
        });

        if result.settings.subtotals {
            result.tree.traverse_mut(
                |node| {
                    let mut tmp = NetworthRow::new(col_count);

                    node.iter_children().for_each(|child| {
                        tmp.add_value(&child.data.data.value);
                        tmp.add_market_value(&child.data.data.market_value);
                    });

                    node.data.data.add_value(&tmp.value);
                    node.data.data.add_market_value(&tmp.market_value);
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
