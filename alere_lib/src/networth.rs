// ??? Account tree is wrong
//    PEA Manu
//    PEA Manu (Brokerange)
//       Children of PEA Manu

use crate::accounts::{AccountId, AccountNameKind};
use crate::commodities::CommodityId;
use crate::multi_values::MultiValue;
use crate::repositories::Repository;
use crate::utils::is_all_same;
use chrono::{DateTime, Local};
use std::collections::{hash_map::Entry, HashMap};

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

pub struct NetworthRow {
    value: Vec<MultiValue>,
    market_value: Vec<MultiValue>,
    account: Option<AccountId>,
    indent: usize,
    has_children: bool,
    full_account_name: String,
}

impl NetworthRow {
    fn new(
        repo: &Repository,
        size: usize,
        account: Option<AccountId>,
        indent: usize,
    ) -> Self {
        NetworthRow {
            value: vec![MultiValue::default(); size],
            market_value: vec![MultiValue::default(); size],
            account,
            full_account_name: match account {
                None => String::new(),
                Some(acc) => repo.get_account_name(acc, AccountNameKind::Full),
            },
            indent,
            has_children: false,
        }
    }

    fn new_with_children(
        repo: &Repository,
        size: usize,
        account: Option<AccountId>,
        indent: usize,
    ) -> Self {
        let mut r = NetworthRow::new(repo, size, account, indent);
        r.has_children = true;
        r
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
    pub fn display_account(
        &self,
        repo: &Repository,
        kind: AccountNameKind,
    ) -> String {
        match self.account {
            None => String::new(),
            Some(acc) => format!(
                "{: <width$}{}",
                "",
                repo.get_account_name(acc, kind),
                width = self.indent,
            ),
        }
    }

    /// Merge two rows
    fn merge(&mut self, right: &NetworthRow) {
        for idx in 0..self.value.len() {
            self.value[idx] += &right.value[idx];
            self.market_value[idx] += &right.market_value[idx];
        }
        self.has_children |= right.has_children;
    }
}

/// A view that shows the value (as of any timestamp) of all user accounts.
/// This ignores all accounts that are not marked as "networth".
pub struct Networth {
    pub lines: Vec<NetworthRow>,
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
            lines: Vec::new(),
            total: NetworthRow::new(repo, col_count, None, 0),
        };
        let mut parent_nodes = HashMap::new();

        for (account, value) in repo.balance(as_of) {
            let mut row = NetworthRow::new(repo, col_count, Some(account), 0);
            row.value = value;
            for (idx, v) in row.value.iter().enumerate() {
                if !v.is_zero() {
                    let mv = market.convert_multi_value(v, &as_of[idx]);
                    result.total.value[idx] += v;
                    result.total.market_value[idx] += &mv;
                    row.market_value[idx] = mv;
                }
            }

            if result.settings.tree {
                let parents = repo.get_account_parents(account);
                row.indent = parents.len();
                for (pidx, p) in parents.into_iter().enumerate() {
                    let n = parent_nodes.entry(p).or_insert_with(|| {
                        result.lines.push(NetworthRow::new_with_children(
                            repo,
                            col_count,
                            Some(p),
                            pidx,
                        ));
                        result.lines.len() - 1
                    });
                    result.lines[*n].has_children = true;
                    if result.settings.subtotals {
                        result.lines[*n].merge(&row);
                    }
                }
            }

            let entry = parent_nodes.entry(account);
            match entry {
                Entry::Occupied(_) => {
                    entry.and_modify(|n| {
                        result.lines[*n].merge(&row);
                    });
                }
                Entry::Vacant(_) => {
                    entry.or_insert_with(|| {
                        result.lines.push(row);
                        result.lines.len() - 1
                    });
                }
            }
        }

        // Filter out rows.  This needs to be done after we have inserted them all
        // above, including the parents, since the values might not be known till
        // that point.
        result.lines.retain(|row|
            // Always keep parent nodes
            row.has_children
            || (
                (!result.settings.hide_zero
                 || !row.value.iter().all(|v| v.is_zero())
                 || !row.market_value.iter().all(|v| v.is_zero()))
                && (!result.settings.hide_all_same
                    || !is_all_same(&row.value)
                    || !is_all_same(&row.market_value))));

        result
            .lines
            .sort_by(|l1, l2| l1.full_account_name.cmp(&l2.full_account_name));
        result
    }
}
