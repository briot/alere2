use crate::{
    global_settings::GlobalSettings,
    tables::{Align, Column, ColumnFooter, Table, Truncate, Width},
};
use alere_lib::{
    accounts::{Account, AccountId, AccountNameDepth},
    networth::{GroupBy, Networth, NetworthRow},
    repositories::Repository,
    times::{Instant, Intv},
    tree_keys::Key,
    trees::NodeData,
};
use anyhow::Result;
use clap::{arg, Arg, ArgAction, ArgMatches};
use console::Term;
use itertools::Itertools;

pub struct Settings {
    // Which columns to show
    column_value: bool,
    column_market: bool,
    column_delta: bool,
    column_market_delta: bool,
    column_delta_to_last: bool,
    column_market_delta_to_last: bool,
    column_price: bool,

    // ???
    // pub column_gain: bool,   // unrealized gains

    // Whether to show percent of total
    column_percent: bool,

    account_names: AccountNameDepth,
}

impl Settings {
    pub fn cli() -> impl IntoIterator<Item = Arg> {
        [arg!(--empty "Show rows with only zero values")
            .action(ArgAction::SetTrue)]
    }
}

pub fn networth_view<F>(
    repo: &Repository,
    args: &ArgMatches,
    globals: &GlobalSettings,
    account_filter: F,
) -> Result<String>
where
    F: FnMut(&(AccountId, &Account)) -> bool,
{
    let nw_settings = alere_lib::networth::Settings {
        hide_zero: !args.get_flag("empty"),
        hide_all_same: false,
        group_by: GroupBy::ParentAccount,
        subtotals: true,
        commodity: globals.commodity,
        elide_boring_accounts: true,
        intervals: vec![
            Intv::UpTo(Instant::EndYearsAgo(2)),
            Intv::UpTo(Instant::MonthsAgo(1)),
            Intv::UpTo(Instant::Now),
        ],
    };
    let view_settings = &crate::networth_view::Settings {
        column_market: true,
        column_value: false,
        column_delta: false,
        column_delta_to_last: false,
        column_price: false,
        column_market_delta: false,
        column_market_delta_to_last: false,
        column_percent: false,
        account_names: AccountNameDepth(1),
    };
    let mut networth =
        Networth::new(repo, nw_settings, globals.reftime, account_filter)?;

    type Data<'a> = NodeData<Key<'a>, NetworthRow>;

    let mv_image = |row: &Data, idx: &usize| {
        row.data.display_value(repo, *idx, &globals.format)
    };
    let market_image = |row: &Data, idx: &usize| {
        row.data.display_market_value(repo, *idx, &globals.format)
    };
    let delta_image = |row: &Data, idx: &usize| {
        row.data.display_delta(repo, *idx, &globals.format)
    };
    let delta_market_image = |row: &Data, idx: &usize| {
        row.data.display_market_delta(repo, *idx, &globals.format)
    };
    let delta_to_last_image = |row: &Data, idx: &usize| {
        row.data.display_delta_to_last(repo, *idx, &globals.format)
    };
    let delta_market_to_last_image = |row: &Data, idx: &usize| {
        row.data
            .display_market_delta_to_last(repo, *idx, &globals.format)
    };
    let node_image = |row: &Data, _idx: &usize| {
        match row.key {
            Key::Account(a) => repo.get_account_name(
                a,
                AccountNameDepth(
                    view_settings.account_names.0 + row.collapse_depth,
                ),
            ),
            Key::Institution(Some(inst)) => inst.name.clone(),
            Key::Institution(None) => "Unknown".to_string(),
            Key::AccountKind(Some(kind)) => kind.name.clone(),
            Key::AccountKind(None) => "Unknown".to_string(),
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
                Column::new(idx, &mv_image)
                    .with_title(&format!("Value {}", ts.descr))
                    .with_align(Align::Right)
                    .with_truncate(Truncate::Left)
                    .with_footer(ColumnFooter::Hide),
            );
        }
        if view_settings.column_market {
            columns.push(
                Column::new(idx, &market_image)
                    .with_title(&format!("Mkt {}", ts.descr))
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
                    Column::new(idx, &delta_image)
                        .with_title("Delta")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if view_settings.column_market_delta {
                columns.push(
                    Column::new(idx, &delta_market_image)
                        .with_title("Delta Mkt")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if view_settings.column_delta_to_last {
                columns.push(
                    Column::new(idx, &delta_to_last_image)
                        .with_title("To lastDelta")
                        .with_align(Align::Right)
                        .with_truncate(Truncate::Left),
                );
            }
            if view_settings.column_market_delta_to_last {
                columns.push(
                    Column::new(idx, &delta_market_to_last_image)
                        .with_title("To last")
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
