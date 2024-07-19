use alere_lib::accounts::AccountNameKind;
use alere_lib::importers::Importer;
use alere_lib::kmymoney::KmyMoneyImporter;
use alere_lib::multi_values::MultiValue;
use std::path::Path;

fn truncate(s: &str, max_width: usize) -> &str {
    s.char_indices()
        .rev()
        .nth(max_width - 1)
        .map_or_else(|| s, |(i, _)| &s[i..])
}

fn main() {
    let kmy = KmyMoneyImporter::default();
    match kmy.import_file(Path::new("./Comptes.kmy")) {
        Err(e) => {
            println!("Error {:?}", e);
        }
        Ok(repo) => {
            const MAX_WIDTH: usize = 30;
            let mut market = repo.market_prices(
                repo.find_commodity("Euro")
            );
            let bal = repo.balance();

            let mut lines = vec![];
            for (account, value) in &bal {
                if !value.is_zero() {
                    lines.push((
                        repo.get_account_name(*account, AccountNameKind::Full),
                        value,
                        market.convert_multi_value(value),
                    ));
                }
            }
            lines.sort_by(|l1, l2| l1.0.cmp(&l2.0));

            let mut total = MultiValue::default();
            for (acc, val, market_val) in lines {
                total += &market_val;
                println!(
                    "{:<0width$} {:>17} {:>17}",
                    truncate(&acc, MAX_WIDTH),
                    repo.display_multi_value(val),
                    repo.display_multi_value(&market_val),
                    width = MAX_WIDTH,
                );
            }
            println!(
                "{:<0width$} {:>17} {:>17}",
                "== Total ==",
                "",
                repo.display_multi_value(&total),
                width = MAX_WIDTH
            );
        }
    }
}
