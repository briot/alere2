// ??? Account tree is wrong
//    PEA Manu
//    PEA Manu (Brokerange)
//       Children of PEA Manu

use crate::tables::{Align, Column, ColumnFooter, Table, Truncate, Width};
use alere_lib::accounts::AccountNameKind;
use alere_lib::commodities::CommodityId;
use alere_lib::multi_values::MultiValue;
use alere_lib::repositories::Repository;
use chrono::{DateTime, Local};
use console::Term;
use itertools::Itertools;
use std::collections::{
    HashMap,
    hash_map::Entry,
};

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

    // Display a tree of accounts
    pub tree: bool,

    // If true, parents' values will also including all their children
    pub subtotals: bool,

    // Currency for market values
    pub commodity: Option<CommodityId>,
}

struct Row {
    value: Vec<MultiValue>,
    market_value: Vec<MultiValue>,
    full_account_name: String,
    short_account_name: String,
    depth: usize,
    has_children: bool,
}

/// A view that shows the value (as of any timestamp) of all user accounts.
/// This ignores all accounts that are not marked as "networth".
pub fn networth_view(
    repo: &Repository,
    as_of: &[DateTime<Local>],
    settings: Settings,
) -> String {

    let col_count = as_of.len();
    let mut market = repo.market_prices(settings.commodity);
    let mut lines = Vec::new();
    let mut total = Row {
        value: vec![MultiValue::default(); col_count],
        market_value: vec![MultiValue::default(); col_count],
        full_account_name: String::new(),
        short_account_name: String::new(),
        depth: 0,
        has_children: false,
    };
    let mut parent_nodes = HashMap::new();

    for (account, value) in repo.balance(as_of) {
        let mut row = Row {
            market_value: vec![MultiValue::default(); col_count],
            value,
            short_account_name: repo
                .get_account_name(account, AccountNameKind::Short),
            full_account_name: repo
                .get_account_name(account, AccountNameKind::Full),
            depth: 0,
            has_children: false,
        };

        for (idx, v) in row.value.iter().enumerate() {
            if !v.is_zero() {
                let market_value = market.convert_multi_value(v, &as_of[idx]);
                total.value[idx] += v;
                total.market_value[idx] += &market_value;
                row.market_value[idx] = market_value;
            }
        }

        if settings.tree {
            let parents = repo.get_account_parents(account);
            row.depth = parents.len();
            for (pidx, p) in parents.into_iter().enumerate() {
                let n = parent_nodes.entry(p).or_insert_with(|| {
                    lines.push(Row {
                        value: vec![MultiValue::default(); col_count],
                        market_value: vec![MultiValue::default(); col_count],
                        short_account_name: repo
                            .get_account_name(p, AccountNameKind::Short),
                        full_account_name: repo
                            .get_account_name(p, AccountNameKind::Full),
                        depth: pidx,
                        has_children: true,
                    });
                    lines.len() - 1
                });
                lines[*n].has_children = true;
                if settings.subtotals {
                    merge(&mut lines[*n], &row);
                }
            }
        }

        let entry = parent_nodes.entry(account);
        match entry {
            Entry::Occupied(_) => {
                entry.and_modify(|n| {
                    merge(&mut lines[*n], &row);
                });
            }
            Entry::Vacant(_) => {
                entry.or_insert_with(|| {
                    lines.push(row);
                    lines.len() - 1
                });
            }
        }
    }

    // Filter out rows.  This needs to be done after we have inserted them all
    // above, including the parents, since the values might not be known till
    // that point.
    lines.retain(|row|
        // Always keep parent nodes
        row.has_children
        || (
            (!settings.hide_zero
             || !row.value.iter().all(|v| v.is_zero())
             || !row.market_value.iter().all(|v| v.is_zero()))
            && (!settings.hide_all_same
                || !is_all_same(&row.value)
                || !is_all_same(&row.market_value))));

    lines.sort_by(|l1, l2| l1.full_account_name.cmp(&l2.full_account_name));

    let mv_image =
        |row: &Row, idx: &usize| repo.display_multi_value(&row.value[*idx]);
    let market_image = |row: &Row, idx: &usize| {
        repo.display_multi_value(&row.market_value[*idx])
    };
    let delta_image = |row: &Row, idx: &usize| {
        repo.display_multi_value(&(&row.value[*idx + 1] - &row.value[*idx]))
    };
    let delta_market_image = |row: &Row, idx: &usize| {
        repo.display_multi_value(
            &(&row.market_value[*idx + 1] - &row.market_value[*idx]),
        )
    };
    let account_image = |row: &Row, _idx: &usize| {
        if settings.tree {
            format!(
                "{: <width$}{}",
                "",
                row.short_account_name,
                width = row.depth
            )
        } else {
            row.full_account_name.clone()
        }
    };

    let mut columns = Vec::new();
    for (pos, (idx, ts)) in as_of.iter().enumerate().with_position() {
        if settings.column_value {
            columns.push(
                Column::new(idx, &mv_image)
                    .with_title(&format!("Value {}", ts.date_naive()))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left)
                    .with_footer(ColumnFooter::Hide),
            );
        }
        if settings.column_market {
            columns.push(
                Column::new(idx, &market_image)
                    .with_title(&format!("Mkt {}", ts.date_naive()))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if let itertools::Position::Last | itertools::Position::Only = pos {
        } else {
            if settings.column_delta {
                columns.push(
                    Column::new(idx, &delta_image)
                        .with_title("Delta")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if settings.column_market_delta {
                columns.push(
                    Column::new(idx, &delta_market_image)
                        .with_title("Delta Mkt")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
        }
    }

    columns.push(
        Column::new(0, &account_image)
            .with_title("Account")
            .with_width(Width::Expand)
            .with_truncate(if settings.tree {
                Truncate::Right
            } else {
                Truncate::Left
            })
            .with_footer(ColumnFooter::Hide),
    );

    let mut table = Table::new(columns).with_col_headers();
    table.add_rows(lines);
    table.add_footer(&total);

    table.to_string(Term::stdout().size().1 as usize)
}

/// Whether the vector contains all-equal elements
fn is_all_same<T: PartialEq>(arr: &[T]) -> bool {
    arr.windows(2).all(|w| w[0] == w[1])
}

/// Merge two rows
fn merge(left: &mut Row, right: &Row) {
    for idx in 0..left.value.len() {
        left.value[idx] += &right.value[idx];
        left.market_value[idx] += &right.market_value[idx];
    }
    left.has_children |= right.has_children;
}

