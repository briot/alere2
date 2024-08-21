use crate::accounts::{AccountId, AccountNameKind};
use crate::commodities::CommodityId;
use crate::multi_values::MultiValue;
use crate::repositories::Repository;
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
    account: Option<AccountId>,
}

impl NetworthRow {
    fn new(
        //        repo: &Repository,
        size: usize,
        account: Option<AccountId>,
    ) -> Self {
        NetworthRow {
            value: vec![MultiValue::default(); size],
            market_value: vec![MultiValue::default(); size],
            account,
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
    pub fn display_account(
        &self,
        repo: &Repository,
        kind: AccountNameKind,
    ) -> String {
        match self.account {
            None => String::new(),
            Some(acc) => repo.get_account_name(acc, kind),
        }
    }

    //    /// Merge two rows
    //    fn merge(&mut self, right: &NetworthRow) {
    //        for idx in 0..self.value.len() {
    //            self.value[idx] += &right.value[idx];
    //            self.market_value[idx] += &right.market_value[idx];
    //        }
    //    }
}

struct NodeList<T>(Vec<AccountTreeNode<T>>);

impl<T> NodeList<T> {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn try_get<F>(
        &mut self,
        repo: &Repository,
        account: AccountId,
        create: &mut F,
        self_depth: usize,
    ) -> &mut AccountTreeNode<T>
    where
        F: FnMut(AccountId) -> T,
    {
        // Go through an index to avoid issues with the borrow checker
        if let Some(i) = self.0.iter().position(|n| n.account == account) {
            &mut self.0[i]
        } else {
            self.0.push(AccountTreeNode::new(
                repo,
                account,
                create(account),
                self_depth + 1,
            ));
            self.0.last_mut().unwrap()
        }
    }

    fn sort_recursive(&mut self) {
        self.0
            .sort_by(|n1, n2| n1.account_name.cmp(&n2.account_name));
        for node in &mut self.0 {
            node.children.sort_recursive();
        }
    }

    fn retain_recursive<F>(&mut self, filter: &mut F)
    where
        F: FnMut(&AccountTreeNode<T>) -> bool,
    {
        for node in &mut self.0 {
            node.children.retain_recursive(filter);
        }
        self.0.retain(|node| filter(node));
    }

    fn traverse_recursive<F>(&self, process: &mut F)
    where
        F: FnMut(&NodeData<T>),
    {
        for node in &self.0 {
            process(&node.data);
            node.children.traverse_recursive(process);
        }
    }
}

pub struct NodeData<T> {
    pub data: T,
    pub depth: usize,
}

pub struct AccountTreeNode<T> {
    account: AccountId,
    account_name: String,
    children: NodeList<T>,
    data: NodeData<T>,
}

impl<T> AccountTreeNode<T> {
    fn new(
        repo: &Repository,
        account: AccountId,
        data: T,
        depth: usize,
    ) -> Self {
        Self {
            account,
            account_name: repo.get_account_name(account, AccountNameKind::Full),
            children: NodeList::new(),
            data: NodeData {
                data,
                depth,
            },
        }
    }

    pub fn has_children(&self) -> bool {
        !self.children.0.is_empty()
    }
}

pub struct AccountTree<T> {
    roots: NodeList<T>,
}

impl<T> Default for AccountTree<T> {
    fn default() -> Self {
        Self {
            roots: NodeList::new(),
        }
    }
}

impl<T> AccountTree<T> {
    pub fn try_get<F>(
        &mut self,
        repo: &Repository,
        account: AccountId,
        parents: &[AccountId], // immediate parent is first in list
        mut create: F,
    ) -> &mut T
    where
        F: FnMut(AccountId) -> T,
    {
        let mut current = &mut self.roots;
        let mut depth = 0_usize;
        for p in parents.iter().rev() {
            current =
                &mut current.try_get(repo, *p, &mut create, depth).children;
            depth += 1;
        }
        &mut current.try_get(repo, account, &mut create, depth).data.data
    }

    pub fn sort(&mut self) {
        self.roots.sort_recursive();
    }

    /// First remove unwanted children, then look at the node itself, so that
    /// the filter can find out whether there remains any children
    pub fn retain<F>(&mut self, mut filter: F)
    where
        F: FnMut(&AccountTreeNode<T>) -> bool,
    {
        self.roots.retain_recursive(&mut filter);
    }

    /// Recursively traverse all nodes, first calling process with the parent
    /// then with all of the children.
    /// It passes the depth of the node (starting at 0 for root nodes) to the
    /// callback.
    pub fn traverse<F>(&self, mut process: F)
    where
        F: FnMut(&NodeData<T>),
    {
        self.roots.traverse_recursive(&mut process);
    }
}

/// A view that shows the value (as of any timestamp) of all user accounts.
/// This ignores all accounts that are not marked as "networth".
pub struct Networth {
    pub tree: AccountTree<NetworthRow>,
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
            tree: AccountTree::default(),
            total: NetworthRow::new(col_count, None),
        };

        for (account, value) in repo.balance(as_of) {
            let parents = repo.get_account_parents(account);
            let row = result.tree.try_get(repo, account, &parents, |acc| {
                NetworthRow::new(col_count, Some(acc))
            });
            row.value = value; // ??? Should merge instead

            for (idx, v) in row.value.iter().enumerate() {
                if !v.is_zero() {
                    let mv = market.convert_multi_value(v, &as_of[idx]);
                    result.total.value[idx] += v;
                    result.total.market_value[idx] += &mv;
                    row.market_value[idx] = mv;
                }
            }
        }

        // Filter out rows.  This needs to be done after we have inserted them all
        // above, including the parents, since the values might not be known till
        // that point.
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

        result.tree.sort();
        result
    }
}
