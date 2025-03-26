use crate::{
    global_settings::GlobalSettings,
    tables::{Align, Column, ColumnFooter, Table, Truncate, Width},
};
use alere_lib::{
    accounts::{Account, AccountNameDepth},
    networth::{Networth, NetworthRow},
    repositories::Repository,
    tree_keys::Key,
    trees::NodeData,
};
use anyhow::Result;
use clap::{Arg, ArgMatches};
use console::Term;
use itertools::Itertools;

#[derive(Default)]
pub struct Settings {
    // Display either the MultiValue, or convert to a Value using
    // the --currency
    pub column_value: bool,
    pub column_delta: bool,
    pub column_delta_to_last: bool,
    pub column_price: bool,
    pub column_percent: bool, //  percent of total
    pub account_names: AccountNameDepth,
}

impl Settings {
    pub fn cli() -> impl IntoIterator<Item = Arg> {
        []
    }
}

pub fn networth_view<F>(
    repo: &Repository,
    _args: &ArgMatches,
    account_filter: F,
    globals: &GlobalSettings,
    networth_settings: alere_lib::networth::Settings,
    view_settings: &crate::networth_view::Settings,
) -> Result<String>
where
    F: FnMut(&Account) -> bool,
{
    let mut networth = Networth::new(
        repo,
        networth_settings,
        globals.reftime,
        account_filter,
    )?;

    type Data<'a> = NodeData<Key, NetworthRow>;

    let market_image = |row: &Data, idx: &usize| {
        row.data.display_market_value(*idx, &globals.format)
    };
    let delta_market_image = |row: &Data, idx: &usize| {
        row.data.display_market_delta(*idx, &globals.format)
    };
    let delta_market_to_last_image = |row: &Data, idx: &usize| {
        row.data.display_market_delta_to_last(*idx, &globals.format)
    };
    let node_image = |row: &Data, _idx: &usize| {
        match &row.key {
            Key::Account(a) => {
                a.name(view_settings.account_names.inc(row.collapse_depth))
            }
            Key::Institution(Some(inst)) => inst.get_name(),
            Key::Institution(None) => "Unknown".to_string(),
            Key::AccountKind(kind) => kind.get_name(),
        }
        //repo.get_account_name(row.key, view_settings.account_names)
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

    for (pos, (idx, ts)) in
        networth.intervals.iter().enumerate().with_position()
    {
        if view_settings.column_value {
            columns.push(
                Column::new(idx, &market_image)
                    .with_title(&ts.descr)
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if view_settings.column_price {
            columns.push(
                Column::new(idx, &price_image)
                    .with_title(&format!("Price {}", ts.descr))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if view_settings.column_percent {
            columns.push(
                Column::new(idx, &percent_image)
                    .with_title("% total")
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left),
            );
        }
        if let itertools::Position::First | itertools::Position::Middle = pos {
            if view_settings.column_delta {
                columns.push(
                    Column::new(idx, &delta_market_image)
                        .with_title("Delta")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if view_settings.column_delta_to_last {
                columns.push(
                    Column::new(idx, &delta_market_to_last_image)
                        .with_title(&format!(
                            "{}-{}",
                            ts.descr,
                            networth.intervals.last().unwrap().descr,
                        ))
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
        }
    }

    let mut table = Table::new(columns, &globals.table).with_col_headers();
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
