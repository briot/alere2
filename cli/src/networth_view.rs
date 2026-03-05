use crate::global_settings::GlobalSettings;
use alere_lib::{
    accounts::{Account, AccountNameDepth},
    networth::{Networth, NetworthRow},
    repositories::Repository,
    tree_keys::Key,
    trees::NodeData,
};
use anyhow::Result;
use clap::Parser;
use itertools::Itertools;
use tabled::{builder::Builder, settings::Style};

#[derive(Parser, Default)]
pub struct Settings {
    // Display either the MultiValue, or convert to a Value using
    // the --currency
    #[clap(skip)]
    pub column_value: bool,
    #[clap(skip)]
    pub column_delta: bool,
    #[clap(skip)]
    pub column_delta_to_last: bool,
    #[clap(skip)]
    pub column_price: bool,
    #[clap(skip)]
    pub column_percent: bool, //  percent of total
    #[clap(skip)]
    pub account_names: AccountNameDepth,
}

pub fn networth_view<F>(
    repo: &Repository,
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

    let node_name = |row: &Data| match &row.key {
        Key::Account(a) => {
            a.name(view_settings.account_names.inc(row.collapse_depth))
        }
        Key::Institution(Some(inst)) => inst.get_name(),
        Key::Institution(None) => "Unknown".to_string(),
        Key::AccountKind(kind) => kind.get_name(),
    };

    // Build header row
    let mut header = vec!["Account".to_string()];
    for (pos, (_, ts)) in networth.intervals.iter().enumerate().with_position()
    {
        if view_settings.column_value {
            header.push(ts.descr.clone());
        }
        if view_settings.column_price {
            header.push(format!("Price {}", ts.descr));
        }
        if view_settings.column_percent {
            header.push("% total".to_string());
        }
        if let itertools::Position::First | itertools::Position::Middle = pos {
            if view_settings.column_delta {
                header.push("Delta".to_string());
            }
            if view_settings.column_delta_to_last {
                header.push(format!(
                    "{}-{}",
                    ts.descr,
                    networth.intervals.last().unwrap().descr,
                ));
            }
        }
    }

    let mut builder = Builder::default();
    builder.push_record(header);

    networth.tree.sort(node_name);

    networth.tree.traverse(
        |node| {
            let indent = "  ".repeat(node.data.depth);
            let mut row = vec![format!("{}{}", indent, node_name(&node.data))];

            for (pos, (idx, _)) in
                networth.intervals.iter().enumerate().with_position()
            {
                if view_settings.column_value {
                    row.push(
                        node.data
                            .data
                            .display_market_value(idx, &globals.format)?,
                    );
                }
                if view_settings.column_price {
                    row.push(node.data.data.display_price(idx)?);
                }
                if view_settings.column_percent {
                    row.push(
                        node.data.data.display_percent(&networth.total, idx)?,
                    );
                }
                if let itertools::Position::First
                | itertools::Position::Middle = pos
                {
                    if view_settings.column_delta {
                        row.push(
                            node.data
                                .data
                                .display_market_delta(idx, &globals.format)?,
                        );
                    }
                    if view_settings.column_delta_to_last {
                        row.push(node.data.data.display_market_delta_to_last(
                            idx,
                            &globals.format,
                        )?);
                    }
                }
            }

            builder.push_record(row);
            Ok(())
        },
        true,
    )?;

    // Add footer
    let mut footer = vec!["Total".to_string()];
    for (pos, (idx, _)) in networth.intervals.iter().enumerate().with_position()
    {
        if view_settings.column_value {
            footer.push(
                networth.total.display_market_value(idx, &globals.format)?,
            );
        }
        if view_settings.column_price {
            footer.push(String::new());
        }
        if view_settings.column_percent {
            footer.push(String::new());
        }
        if let itertools::Position::First | itertools::Position::Middle = pos {
            if view_settings.column_delta {
                footer.push(
                    networth
                        .total
                        .display_market_delta(idx, &globals.format)?,
                );
            }
            if view_settings.column_delta_to_last {
                footer.push(
                    networth
                        .total
                        .display_market_delta_to_last(idx, &globals.format)?,
                );
            }
        }
    }
    builder.push_record(footer);

    let mut table = builder.build();
    table.with(Style::modern());
    Ok(table.to_string())
}
