use crate::tables::{Align, Column, ColumnFooter, Table, Truncate, Width};
use alere_lib::networth::{Networth, NetworthRow};
use alere_lib::repositories::Repository;
use console::Term;
use itertools::Itertools;

/// A view that shows the value (as of any timestamp) of all user accounts.
/// This ignores all accounts that are not marked as "networth".
pub fn networth_view(repo: &Repository, networth: Networth) -> String {
    let mv_image = |row: &NetworthRow, idx: &usize| {
        repo.display_multi_value(&row.value[*idx])
    };
    let market_image = |row: &NetworthRow, idx: &usize| {
        repo.display_multi_value(&row.market_value[*idx])
    };
    let delta_image = |row: &NetworthRow, idx: &usize| {
        repo.display_multi_value(&(&row.value[*idx + 1] - &row.value[*idx]))
    };
    let delta_market_image = |row: &NetworthRow, idx: &usize| {
        repo.display_multi_value(
            &(&row.market_value[*idx + 1] - &row.market_value[*idx]),
        )
    };
    let account_image = |row: &NetworthRow, _idx: &usize| match row.account {
        None => String::new(),
        Some(acc) => format!(
            "{: <width$}{}",
            "",
            repo.get_account_name(acc, networth.settings.account_names),
            width = row.indent,
        ),
    };

    let mut columns = Vec::new();
    for (pos, (idx, ts)) in networth.as_of.iter().enumerate().with_position() {
        if networth.settings.column_value {
            columns.push(
                Column::new(idx, &mv_image)
                    .with_title(&format!("Value {}", ts.date_naive()))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left)
                    .with_footer(ColumnFooter::Hide),
            );
        }
        if networth.settings.column_market {
            columns.push(
                Column::new(idx, &market_image)
                    .with_title(&format!("Mkt {}", ts.date_naive()))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if let itertools::Position::First | itertools::Position::Middle = pos {
            if networth.settings.column_delta {
                columns.push(
                    Column::new(idx, &delta_image)
                        .with_title("Delta")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if networth.settings.column_market_delta {
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
            .with_truncate(if networth.settings.tree {
                // Need to show the indentation always
                Truncate::Right
            } else {
                Truncate::Left
            })
            .with_footer(ColumnFooter::Hide),
    );

    let mut table = Table::new(columns).with_col_headers();
    table.add_rows(networth.lines);
    table.add_footer(&networth.total);

    table.to_string(Term::stdout().size().1 as usize)
}
