use crate::tables::{Align, Column, ColumnFooter, Table, Truncate, Width};
use alere_lib::accounts::AccountId;
use alere_lib::networth::{Networth, NetworthRow};
use alere_lib::repositories::Repository;
use alere_lib::trees::NodeData;
use console::Term;
use itertools::Itertools;

pub fn networth_view(repo: &Repository, networth: Networth) -> String {
    type Data = NodeData<AccountId, NetworthRow>;

    let mv_image = |row: &Data, idx: &usize| row.data.display_value(repo, *idx);
    let market_image =
        |row: &Data, idx: &usize| row.data.display_market_value(repo, *idx);
    let delta_image =
        |row: &Data, idx: &usize| row.data.display_delta(repo, *idx);
    let delta_market_image =
        |row: &Data, idx: &usize| row.data.display_market_delta(repo, *idx);
    let account_image = |row: &Data, _idx: &usize| {
        format!(
            "{: <width$}{}",
            "",
            repo.get_account_name(row.key, networth.settings.account_names),
            width = row.depth,
        )
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
    networth
        .tree
        .traverse(|node| table.add_row(&node.data), true);

    table.add_footer(&Data {
        data: networth.total.clone(),
        depth: 0,
        key: AccountId(0), //  ??? irrelevant
    });
    table.to_string(Term::stdout().size().1 as usize)
}
