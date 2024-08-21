use crate::tables::{Align, Column, ColumnFooter, Table, Truncate, Width};
use alere_lib::networth::{Networth, NetworthRow};
use alere_lib::repositories::Repository;
use console::Term;
use itertools::Itertools;

pub fn networth_view(repo: &Repository, networth: Networth) -> String {
    let mv_image =
        |row: &NetworthRow, idx: &usize| row.display_value(repo, *idx);
    let market_image =
        |row: &NetworthRow, idx: &usize| row.display_market_value(repo, *idx);
    let delta_image =
        |row: &NetworthRow, idx: &usize| row.display_delta(repo, *idx);
    let delta_market_image =
        |row: &NetworthRow, idx: &usize| row.display_market_delta(repo, *idx);
    let account_image = |row: &NetworthRow, _idx: &usize| {
        row.display_account(repo, networth.settings.account_names)
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
