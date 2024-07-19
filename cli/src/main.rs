use alere_lib::accounts::AccountNameKind;
use alere_lib::importers::Importer;
use alere_lib::kmymoney::KmyMoneyImporter;
use alere_lib::multi_values::{MultiValue, Value};
use rust_decimal::Decimal;
use std::path::Path;

fn trunc_keep_last(s: &str, max_width: usize) -> &str {
    s.char_indices()
        .rev()
        .nth(max_width - 1)
        .map_or_else(|| s, |(i, _)| &s[i..])
}
fn trunc_keep_first(s: &str, max_width: usize) -> &str {
    s.char_indices()
        .nth(max_width)
        .map_or_else(|| s, |(i, _)| &s[..i])
}

fn main() {
    let kmy = KmyMoneyImporter::default();
    match kmy.import_file(Path::new("./Comptes.kmy")) {
        Err(e) => {
            println!("Error {:?}", e);
        }
        Ok(repo) => {
            const COL_ACCOUNT: usize = 30;
            const COL_VALUE: usize = 17;

            let mut market = repo.market_prices(repo.find_commodity("Euro"));
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
                    "{:<0awidth$} {:>0vwidth$} {:>0vwidth$} {:>0vwidth$}",
                    trunc_keep_last(&acc, COL_ACCOUNT),
                    trunc_keep_first(&repo.display_multi_value(val), COL_VALUE),
                    trunc_keep_first(
                        &repo.display_multi_value(&market_val),
                        COL_VALUE
                    ),
                    trunc_keep_first(
                        &market.get_prices(val)
                            .iter()
                            .map(|v| repo.display_value(v))
                            .collect::<Vec<String>>()
                            .join(" + "),
                        COL_VALUE,
                    ),
                    awidth = COL_ACCOUNT,
                    vwidth = COL_VALUE,
                );
            }
            println!(
                "{:<0awidth$} {:>0vwidth$} {:>0vwidth$}",
                "== Total ==",
                "",
                trunc_keep_first(&repo.display_multi_value(&total), COL_VALUE),
                awidth = COL_ACCOUNT,
                vwidth = COL_VALUE,
            );
        }
    }
}
