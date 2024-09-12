use crate::tables::{Align, Column, ColumnFooter, Table, Truncate, Width};
use alere_lib::accounts::AccountNameDepth;
use alere_lib::networth::{Networth, NetworthRow};
use alere_lib::repositories::Repository;
use alere_lib::tree_keys::Key;
use alere_lib::trees::NodeData;
use anyhow::Result;
use console::Term;
use itertools::Itertools;

pub struct Settings {
    // Which columns to show
    pub column_value: bool,
    pub column_market: bool,
    pub column_delta: bool,
    pub column_market_delta: bool,
    pub column_delta_to_last: bool,
    pub column_market_delta_to_last: bool,
    pub column_price: bool,

    // Whether to show percent of total
    pub column_percent: bool,

    pub account_names: AccountNameDepth,
    pub table: crate::tables::Settings,
}

pub fn networth_view(
    repo: &Repository,
    mut networth: Networth,
    settings: Settings,
) -> Result<String> {
    type Data<'a> = NodeData<Key<'a>, NetworthRow>;

    let mv_image = |row: &Data, idx: &usize| row.data.display_value(repo, *idx);
    let market_image =
        |row: &Data, idx: &usize| row.data.display_market_value(repo, *idx);
    let delta_image =
        |row: &Data, idx: &usize| row.data.display_delta(repo, *idx);
    let delta_market_image =
        |row: &Data, idx: &usize| row.data.display_market_delta(repo, *idx);
    let delta_to_last_image =
        |row: &Data, idx: &usize| row.data.display_delta_to_last(repo, *idx);
    let delta_market_to_last_image = |row: &Data, idx: &usize| {
        row.data.display_market_delta_to_last(repo, *idx)
    };
    let node_image = |row: &Data, _idx: &usize| {
        match row.key {
            Key::Account(a) => repo.get_account_name(
                a,
                AccountNameDepth(settings.account_names.0 + row.collapse_depth),
            ),
            Key::Institution(Some(inst)) => inst.name.clone(),
            Key::Institution(None) => "Unknown".to_string(),
            Key::AccountKind(Some(kind)) => kind.name.clone(),
            Key::AccountKind(None) => "Unknown".to_string(),
        }
        //repo.get_account_name(row.key, settings.account_names)
    };
    let price_image = |row: &Data, idx: &usize| row.data.display_price(*idx);
    let percent_image = |row: &Data, idx: &usize| {
        row.data.display_percent(&networth.total, *idx)
    };

    let mut columns = Vec::new();
    columns.push(
        Column::new(0, &node_image)
            .show_indent()
            .with_title("Account")
            .with_width(Width::ExpandWithMin(8))
            .with_truncate(Truncate::Left)
            .with_footer(ColumnFooter::Hide),
    );

    for (pos, (idx, ts)) in networth.as_of.iter().enumerate().with_position() {
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
        if settings.column_price {
            columns.push(
                Column::new(idx, &price_image)
                    .with_title(&format!("Price {}", ts.date_naive()))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if settings.column_percent {
            columns.push(
                Column::new(idx, &percent_image)
                    .with_title("% total")
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if let itertools::Position::First | itertools::Position::Middle = pos {
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
            if settings.column_delta_to_last {
                columns.push(
                    Column::new(idx, &delta_to_last_image)
                        .with_title("To lastDelta")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if settings.column_market_delta_to_last {
                columns.push(
                    Column::new(idx, &delta_market_to_last_image)
                        .with_title("To last")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
        }
    }

    let mut table = Table::new(columns, settings.table).with_col_headers();
    networth.tree.sort(|nodedata| node_image(nodedata, &0));

    networth.tree.traverse(
        |node| {
            table.add_row(&node.data, node.data.depth);
            Ok(())
        },
        true,
    )?;

    table.add_footer(&Data::new(
        Key::Institution(None), //  ??? irrelevant
        networth.total.clone(),
    ));
    Ok(table.to_string(Term::stdout().size().1 as usize))
}
