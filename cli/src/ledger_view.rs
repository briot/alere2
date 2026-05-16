use alere_lib::{
    accounts::AccountNameDepth,
    multi_values::{MultiValue, Operation},
    repositories::Repository,
    times::Instant,
};
use anyhow::Result;

use crate::global_settings::GlobalSettings;

#[allow(clippy::too_many_arguments)]
pub fn ledger_view(
    repo: &Repository,
    settings: &GlobalSettings,
    account_filter: Option<&str>,
    short_name: bool,
    columns: Option<&Vec<String>>,
    since: Option<&Instant>,
    before: Option<&Instant>,
    filter: Option<&str>,
) -> Result<String> {
    use tabled::builder::Builder;

    // Convert filter pattern to regex (support * wildcard)
    let filter_regex = filter
        .map(|f| {
            let pattern = regex::escape(f).replace(r"\*", ".*");
            regex::RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .build()
        })
        .transpose()?;

    // Default columns if none specified
    let default_cols = vec![
        "balance".to_string(),
        "memo".to_string(),
        "splits".to_string(),
    ];
    let cols = columns.unwrap_or(&default_cols);

    // Parse column options
    let show_splits = cols.iter().any(|c| c.eq_ignore_ascii_case("splits"));

    // Build ordered column list (Date, Account, Amount are always first)
    let mut header = vec!["Date", "Account", "Amount"];
    for col in cols.iter() {
        match col.to_lowercase().as_str() {
            "balance" if filter.is_none() => header.push("Balance"),
            "payee" => header.push("Payee"),
            "what" => header.push("What"),
            "memo" => header.push("Memo"),
            _ => {}
        }
    }

    // Parse date filters
    let since_date = since.and_then(|i| i.to_time(settings.reftime).ok());
    let before_date = before.and_then(|i| i.to_time(settings.reftime).ok());

    let mut transactions: Vec<_> = repo.transactions().iter().collect();
    transactions.sort_by_key(|tx| {
        tx.splits().first().map(|s| s.post_ts).unwrap_or_default()
    });

    let display_depth = if short_name {
        AccountNameDepth::basename()
    } else {
        AccountNameDepth::unlimited()
    };

    let mut builder = Builder::default();
    builder.push_record(header);

    let mut running_total = MultiValue::default();

    'transactions: for tx in transactions {
        let splits = tx.splits();
        let memo = tx.memo();

        // Always match against full name
        let matches = if let Some(filter) = account_filter {
            splits.iter().any(|s| {
                s.account
                    .name(AccountNameDepth::unlimited())
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            })
        } else {
            true
        };

        if !matches {
            continue;
        }

        let mut valid_splits = vec![];

        for s in splits.iter() {
            // Update running total with all splits that match the account
            // filter (so an internal transfer, for instance, would not move
            // the total if both accounts are valid for the filter)
            let for_total = if let Some(filter) = account_filter {
                s.account
                    .name(AccountNameDepth::unlimited())
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            } else {
                s.account
                    .name(AccountNameDepth::unlimited())
                    .starts_with("Asset:")
            };

            // Update running total
            let amount_mv = if for_total {
                let v = match &s.operation {
                    Operation::Credit(v) => v.clone(),
                    Operation::BuyAmount { qty, .. } => {
                        MultiValue::new(qty.amount, &qty.commodity)
                    }
                    Operation::BuyPrice { qty, .. } => {
                        MultiValue::new(qty.amount, &qty.commodity)
                    }
                    Operation::AddShares { qty } => {
                        MultiValue::new(qty.amount, &qty.commodity)
                    }
                    Operation::Reinvest { shares, .. } => shares.clone(),
                    Operation::Dividend | Operation::Split { .. } => {
                        MultiValue::default()
                    }
                };
                running_total += &v;
                v
            } else {
                MultiValue::zero()
            };

            let ts = s.post_ts;
            let after_since = since_date.is_none_or(|d| ts >= d);
            let before_limit = before_date.is_none_or(|d| ts <= d);
            if for_total && after_since && before_limit {
                valid_splits.push((s, amount_mv));
            }
        }

        // Now chose which of the remaining splits with be used as the main
        // one: it should be one that matches the filter, and among those it
        // doesn't matter which one.  If none match the filter, hide the
        // transaction.
        for (s, amount_mv) in valid_splits.iter() {
            let date_str = s.post_ts.format("%Y-%m-%d").to_string();
            let account_str = s.account.name(display_depth);
            let account_full = s.account.name(AccountNameDepth::unlimited());
            let amount_str = amount_mv.display(&settings.format);

            let balance_str = if settings.commodity.is_some() {
                let mut prices = repo.market_prices(settings.commodity.clone());
                let converted =
                    prices.convert_multi_value(&running_total, &s.post_ts);
                converted.display(&settings.format)
            } else {
                running_total.display(&settings.format)
            };

            let memo_str =
                memo.as_ref().map(|s| s.to_string()).unwrap_or_default();
            let payee_str = tx
                .payee()
                .map(|p| p.get_name().to_string())
                .unwrap_or_default();
            let what_str = if !memo_str.is_empty() {
                memo_str.clone()
            } else {
                payee_str.clone()
            };

            let matches = if let Some(ref regex) = filter_regex {
                regex.is_match(&date_str)
                    || regex.is_match(&account_full)
                    || regex.is_match(&amount_str)
                    || regex.is_match(&balance_str)
                    || regex.is_match(&memo_str)
                    || regex.is_match(&payee_str)
                    || regex.is_match(&what_str)
            } else {
                true
            };

            if matches {
                // Build row in column order
                let mut row = vec![date_str, account_str, amount_str];
                for col in cols.iter() {
                    match col.to_lowercase().as_str() {
                        "balance" if filter.is_none() => {
                            row.push(balance_str.clone())
                        }
                        "payee" => row.push(payee_str.clone()),
                        "what" => row.push(what_str.clone()),
                        "memo" => row.push(memo_str.clone()),
                        _ => {}
                    }
                }
                builder.push_record(row);

                // Other splits (indented)
                if show_splits {
                    for split in splits.iter() {
                        // if split == s {
                        //     continue;
                        // }
                        let amount_str = match &split.operation {
                            Operation::Credit(v) => v.display(&settings.format),
                            Operation::BuyAmount { qty, .. } => {
                                qty.display(&settings.format)
                            }
                            Operation::BuyPrice { qty, .. } => {
                                qty.display(&settings.format)
                            }
                            Operation::AddShares { qty } => {
                                qty.display(&settings.format)
                            }
                            Operation::Reinvest { shares, .. } => {
                                shares.display(&settings.format)
                            }
                            Operation::Dividend => "dividend".to_string(),
                            Operation::Split { ratio, .. } => {
                                format!("split {}", ratio)
                            }
                        };

                        let mut row = vec![
                            String::new(),
                            format!("  {}", split.account.name(display_depth)),
                            amount_str,
                        ];
                        for col in cols.iter() {
                            match col.to_lowercase().as_str() {
                                "balance" if filter.is_none() => {
                                    row.push(String::new())
                                }
                                "payee" | "what" | "memo" => {
                                    row.push(String::new())
                                }
                                _ => {}
                            }
                        }
                        builder.push_record(row);
                    }
                }
                continue 'transactions; // do not add a second row for this transaction
            }
        }
    }

    Ok(settings.finalize_table(builder, None, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alere_lib::{importers::Importer, kmymoney::KmyMoneyImporter};
    use chrono::Local;
    use futures::executor::block_on;
    use std::str::FromStr;

    fn create_test_data() -> Result<kmy_editor::KmyEditor> {
        let mut editor = kmy_editor::KmyEditor::new()?;

        // Add currency
        editor.add_currency("EUR", "Euro", "€")?;

        // Add accounts
        let checking = editor.add_account("Checking", "1", "EUR")?;
        let savings = editor.add_account("Savings", "1", "EUR")?;
        let equity =
            editor.add_standard_account("Equity", "Equity", "16", "EUR")?;
        let expense =
            editor.add_standard_account("Expense", "Expense", "13", "EUR")?;
        let income =
            editor.add_standard_account("Income", "Income", "12", "EUR")?;

        // Add payee
        let grocery_store = editor.add_payee("Grocery Store")?;

        // Transaction 1: Opening balance
        let t1 = editor.add_transaction(
            "2024-01-01",
            Some("Opening Checking"),
            "EUR",
        )?;
        editor.add_split(&t1, 0, &checking, "2000/1", "2024-01-01", None)?;
        editor.add_split(&t1, 1, &equity, "-2000/1", "2024-01-01", None)?;

        // Transaction 2: Transfer to savings
        let t2 = editor.add_transaction(
            "2024-03-15",
            Some("From checking"),
            "EUR",
        )?;
        editor.add_split(&t2, 0, &checking, "-500/1", "2024-03-15", None)?;
        editor.add_split(&t2, 1, &savings, "500/1", "2024-03-15", None)?;

        // Transaction 3: Expense
        let t3 = editor.add_transaction(
            "2024-06-20",
            Some("Weekly shopping"),
            "EUR",
        )?;
        editor.add_split(&t3, 0, &checking, "-300/1", "2024-06-20", None)?;
        editor.add_split(&t3, 1, &expense, "300/1", "2024-06-20", None)?;

        // Transaction 4: Income
        let t4 = editor.add_transaction(
            "2025-01-15",
            Some("Monthly salary"),
            "EUR",
        )?;
        editor.add_split(&t4, 0, &checking, "2000/1", "2025-01-15", None)?;
        editor.add_split(&t4, 1, &income, "-2000/1", "2025-01-15", None)?;

        // Transaction 5: Expense with payee but no memo
        let t5 = editor.add_transaction("2025-02-10", None, "EUR")?;
        editor.add_split(
            &t5,
            0,
            &checking,
            "-150/1",
            "2025-02-10",
            Some(&grocery_store),
        )?;
        editor.add_split(&t5, 1, &expense, "150/1", "2025-02-10", None)?;

        Ok(editor)
    }

    fn load_test_repo() -> Repository {
        let editor = create_test_data().unwrap();
        let mut kmy = KmyMoneyImporter::default();
        block_on(kmy.import_file(editor.path(), |_, _| {})).unwrap()
    }

    fn test_settings() -> GlobalSettings {
        let mut settings = GlobalSettings::default();
        settings.reftime = Local::now();
        settings
    }

    #[test]
    fn test_ledger_default() {
        let repo = load_test_repo();
        let settings = test_settings();
        let output = ledger_view(
            &repo,
            &settings,
            Some("checking"),
            false,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert!(output.contains("Checking"));
        assert!(output.contains("2,000"));
        assert!(output.contains("Opening Checking"));
        assert!(output.contains("Balance"));
        assert!(output.contains("Memo"));
    }

    #[test]
    fn test_ledger_with_payee_column() {
        let repo = load_test_repo();
        let settings = test_settings();
        let columns = vec!["payee".to_string()];
        let output = ledger_view(
            &repo,
            &settings,
            Some("checking"),
            false,
            Some(&columns),
            None,
            None,
            None,
        )
        .unwrap();

        assert!(output.contains("Payee"));
        assert!(!output.contains("Balance"));
        assert!(!output.contains("Memo"));
    }

    #[test]
    fn test_ledger_with_what_column() {
        let repo = load_test_repo();
        let settings = test_settings();
        let columns = vec!["what".to_string()];
        let output = ledger_view(
            &repo,
            &settings,
            Some("checking"),
            false,
            Some(&columns),
            None,
            None,
            None,
        )
        .unwrap();

        assert!(output.contains("What"));
        assert!(output.contains("Opening Checking"));
        assert!(!output.contains("Balance"));
    }

    #[test]
    fn test_ledger_since_filter() {
        let repo = load_test_repo();
        let settings = test_settings();
        let since = Instant::from_str("start of 3 years ago").unwrap();
        let output = ledger_view(
            &repo,
            &settings,
            Some("checking"),
            false,
            None,
            Some(&since),
            None,
            None,
        )
        .unwrap();

        assert!(output.contains("2024-01-01"));
        assert!(output.contains("2025-01-15"));
    }

    #[test]
    fn test_ledger_before_filter() {
        let repo = load_test_repo();
        let settings = test_settings();
        let before = Instant::from_str("1 years ago").unwrap();
        let output = ledger_view(
            &repo,
            &settings,
            Some("checking"),
            false,
            None,
            None,
            Some(&before),
            None,
        )
        .unwrap();

        // All transactions are more than 1 year ago, so should show
        assert!(output.contains("2024-01-01"));
        assert!(output.contains("2025-01-15"));
    }

    #[test]
    fn test_ledger_multiple_columns() {
        let repo = load_test_repo();
        let settings = test_settings();
        let columns = vec![
            "balance".to_string(),
            "what".to_string(),
            "payee".to_string(),
        ];
        let output = ledger_view(
            &repo,
            &settings,
            Some("checking"),
            false,
            Some(&columns),
            None,
            None,
            None,
        )
        .unwrap();

        assert!(output.contains("Balance"));
        assert!(output.contains("What"));
        assert!(output.contains("Payee"));
        assert!(!output.contains("Memo"));
    }

    #[test]
    fn test_ledger_what_shows_payee_when_no_memo() {
        let repo = load_test_repo();
        let settings = test_settings();
        let columns = vec!["what".to_string(), "payee".to_string()];
        let output = ledger_view(
            &repo,
            &settings,
            Some("checking"),
            false,
            Some(&columns),
            None,
            None,
            None,
        )
        .unwrap();

        // Transaction with memo should show memo in What column
        assert!(output.contains("Opening Checking"));

        // Transaction without memo but with payee should show payee in What column
        assert!(output.contains("Grocery Store"));

        // Verify the transaction with payee exists
        assert!(output.contains("2025-02-10"));
    }
}
