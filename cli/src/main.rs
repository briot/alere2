pub mod tables;

use crate::tables::{Align, Column, Table, Truncate, Width};
use alere_lib::accounts::{AccountId, AccountNameKind};
use alere_lib::importers::Importer;
use alere_lib::kmymoney::KmyMoneyImporter;
use alere_lib::repositories::{MarketPrices, Repository};
use alere_lib::multi_values::MultiValue;
use anyhow::Result;
use chrono::{DateTime, Local};
use console::Term;
use futures::executor::block_on;
use indicatif::{ProgressBar, ProgressStyle};
use std::cmp::Ordering;
use std::path::Path;

trait Cell {
    fn header(&self) -> Column;
    fn cell_content(&self) -> String;
}

//-----------------------------------------------------------
//--  CellValue
//-----------------------------------------------------------
struct CellValue {
    str_value: String,
}
impl CellValue {
    pub fn new(repo: &Repository, value: &MultiValue) -> Self {
        Self { str_value: repo.display_multi_value(value) }
    }
}
impl Cell for CellValue {
    fn header(&self) -> Column {
        Column::default()
            .with_align(Align::Right)
            .with_truncate(Truncate::Left)
    }

    fn cell_content(&self) -> String {
        self.str_value.clone()
    }
}

//-----------------------------------------------------------
//--  CellFullAccount
//-----------------------------------------------------------

#[derive(PartialEq, Eq)]
struct CellFullAccount {
    name: String,
}
impl CellFullAccount {
    pub fn new(repo: &Repository, account: AccountId) -> Self {
        Self { name: repo.get_account_name(account, AccountNameKind::Full)}
    }

}
impl Cell for CellFullAccount {
    fn header(&self) -> Column {
        Column::default()
            .with_truncate(Truncate::Left)
            .with_width(Width::Expand)
    }

    fn cell_content(&self) -> String {
        self.name.clone()
    }
}
impl std::cmp::PartialOrd for CellFullAccount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.name.cmp(&other.name))
    }
}

//-----------------------------------------------------------
//--  CellMarketValue
//-----------------------------------------------------------
struct CellMarketValue {
    str_value: String,
}
impl CellMarketValue {
    pub fn new(repo: &Repository, market: &mut MarketPrices, when: &DateTime<Local>, value: &MultiValue) -> Self {
        let market_val = market.convert_multi_value(value, when);
        Self { str_value: repo.display_multi_value(&market_val) }
    }

}
impl Cell for CellMarketValue {
    fn header(&self) -> Column {
        Column::default()
            .with_align(Align::Right)
            .with_truncate(Truncate::Left)
    }

    fn cell_content(&self) -> String {
        self.str_value.clone()
    }
}


fn main() -> Result<()> {
    let stdout = Term::stdout();

    let progress = ProgressBar::new(1) //  we do not know the length
        .with_style(
            ProgressStyle::with_template(
                "[{pos:2}/{len:2}] {msg} {wide_bar} {elapsed_precise}",
            )
            .unwrap(),
        )
        .with_message("importing kmy");

    let mut kmy = KmyMoneyImporter::default();
    let repo = block_on(kmy.import_file(
        Path::new("./Comptes.kmy"),
        |current, max| {
            progress.set_length(max);
            progress.set_position(current);
        },
    ))?;

    let bal = repo.balance();
    let now = Local::now();
    let mut market = repo.market_prices(repo.find_commodity("Euro"));

    let mut lines: Vec<Vec<Box<dyn Cell>>> = vec![];
    for (account, value) in &bal {
        if !value.is_zero() {
            lines.push(vec![
                Box::new(CellValue::new(&repo, value)),
                Box::new(CellMarketValue::new(&repo, &mut market, &now, value)),
                Box::new(CellFullAccount::new(&repo, *account)),
            ]);
        }
    }
//    lines.sort_by(|l1, l2| l1[1].cmp(l2[1]));

    progress.finish_and_clear();

    let mut table = Table::default();
    let columns = lines.first().unwrap();
    for c in columns {
        table.add_column(c.header());
    }

    let mut total = MultiValue::default();
    for cells in lines {
//        total += &market_val;
        table.add_row(cells.iter().map(|c| c.cell_content()).collect()
        );
    }
    table.add_sep();
    table.add_row(vec![
        repo.display_multi_value(&total),
        "".into(),
        "Total".into(),
    ]);

    let table = table.to_string(stdout.size().1 as usize);
    println!("{}", table);

    Ok(())
}
