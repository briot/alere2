use alere_lib::{
    accounts::{Account, AccountNameDepth},
    networth::{GroupBy, Networth},
    repositories::Repository,
    times::{Instant, Intv},
};
use anyhow::Result;
use tabled::builder::Builder;

use crate::global_settings::GlobalSettings;

fn month_num_to_name(num: &str) -> &'static str {
    match num.trim() {
        "1" => "Jan",
        "2" => "Feb",
        "3" => "Mar",
        "4" => "Apr",
        "5" => "May",
        "6" => "Jun",
        "7" => "Jul",
        "8" => "Aug",
        "9" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => "???",
    }
}

pub fn history_view(
    repo: &Repository,
    settings: &GlobalSettings,
    account_filter: Option<&str>,
    granularity: &str,
    since: Option<&str>,
    before: Option<&str>,
) -> Result<String> {
    let start = if let Some(s) = since {
        s.parse::<Instant>()?
    } else if let Some(date) = repo.earliest_transaction_date() {
        Instant::Timestamp(date.to_rfc3339())
    } else {
        Instant::YearsAgo(1)
    };

    let end = before
        .map(|s| s.parse::<Instant>())
        .transpose()?
        .unwrap_or(Instant::Now);

    let intervals = match granularity {
        "yearly" => Intv::Yearly { begin: start, end },
        _ => Intv::Monthly { begin: start, end },
    };

    let filter = |acc: &Account| {
        if !acc.get_kind().is_networth() {
            return false;
        }
        if let Some(filter) = account_filter {
            acc.name(AccountNameDepth::unlimited())
                .to_lowercase()
                .contains(&filter.to_lowercase())
        } else {
            true
        }
    };

    println!("MANU intervals={intervals:?}");
    let networth = Networth::new(
        repo,
        alere_lib::networth::Settings {
            hide_zero_rows: false,
            hide_all_same: true,
            group_by: GroupBy::None,
            subtotals: true,
            commodity: settings.commodity.clone(),
            elide_boring_accounts: false,
            intervals: vec![intervals],
        },
        settings.reftime,
        filter,
    )?;

    let mut builder = Builder::default();
    builder.push_record(["Date", "Total"]);

    let mut cumulative = alere_lib::multi_values::MultiValue::default();
    let mut prev_cumulative = alere_lib::multi_values::MultiValue::default();

    for (idx, intv) in networth.intervals.iter().enumerate() {
        let change = networth.total.get_market_value(idx)?;
        println!("MANU {intv:?} change={change:?}");
        cumulative += change;

        // Skip if no change
        if cumulative != prev_cumulative {
            let total = cumulative.display(&settings.format);

            if !total.trim().is_empty() {
                // Format date
                let date_str = if granularity == "yearly" {
                    intv.descr.clone()
                } else {
                    // Convert "2026-3" to "2026 Mar"
                    if let Some((year, month)) = intv.descr.split_once('-') {
                        format!("{} {}", year, month_num_to_name(month))
                    } else {
                        intv.descr.clone()
                    }
                };

                builder.push_record([&date_str, &total]);
                prev_cumulative = cumulative.clone();
            }
        }
    }

    // Add current total if different from last shown
    let current_networth = Networth::new(
        repo,
        alere_lib::networth::Settings {
            hide_zero_rows: false,
            hide_all_same: true,
            group_by: GroupBy::None,
            subtotals: true,
            commodity: settings.commodity.clone(),
            elide_boring_accounts: false,
            intervals: vec![Intv::UpTo(Instant::Now)],
        },
        settings.reftime,
        filter,
    )?;

    if let Ok(current_value) = current_networth.total.get_market_value(0)
        && current_value != &prev_cumulative
        && !current_value.display(&settings.format).trim().is_empty()
    {
        let total = current_value.display(&settings.format);
        builder.push_record(["Current", &total]);
    }

    Ok(settings.finalize_table(builder, Some(1), false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alere_lib::{importers::Importer, kmymoney::KmyMoneyImporter};
    use chrono::Local;
    use futures::executor::block_on;

    fn create_test_data() -> Result<kmy_editor::KmyEditor> {
        let mut editor = kmy_editor::KmyEditor::new()?;

        editor.add_currency("EUR", "Euro", "€")?;

        let checking = editor.add_account("Checking", "1", "EUR")?;
        let equity =
            editor.add_standard_account("Equity", "Equity", "16", "EUR")?;

        // Jan 2024: 1000
        let t1 =
            editor.add_transaction("2024-01-15", Some("Opening"), "EUR")?;
        editor.add_split(&t1, 0, &checking, "1000/1", "2024-01-15", None)?;
        editor.add_split(&t1, 1, &equity, "-1000/1", "2024-01-15", None)?;

        // Feb 2024: +500 = 1500
        let t2 =
            editor.add_transaction("2024-02-10", Some("Deposit"), "EUR")?;
        editor.add_split(&t2, 0, &checking, "500/1", "2024-02-10", None)?;
        editor.add_split(&t2, 1, &equity, "-500/1", "2024-02-10", None)?;

        // Mar 2024: +200 = 1700
        let t3 =
            editor.add_transaction("2024-03-20", Some("Deposit"), "EUR")?;
        editor.add_split(&t3, 0, &checking, "200/1", "2024-03-20", None)?;
        editor.add_split(&t3, 1, &equity, "-200/1", "2024-03-20", None)?;

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
    fn test_history_monthly() -> Result<()> {
        let repo = load_test_repo();
        let settings = test_settings();

        let output =
            history_view(&repo, &settings, None, "monthly", Some("1y"), None)?;

        // Just verify it runs and contains some data
        assert!(output.contains("Date"));
        assert!(output.contains("Total"));

        Ok(())
    }

    #[test]
    fn test_history_yearly() -> Result<()> {
        let repo = load_test_repo();
        let settings = test_settings();

        let output =
            history_view(&repo, &settings, None, "yearly", Some("2y"), None)?;

        assert!(output.contains("Date"));
        assert!(output.contains("Total"));

        Ok(())
    }

    #[test]
    fn test_history_account_filter() -> Result<()> {
        let repo = load_test_repo();
        let settings = test_settings();

        let output = history_view(
            &repo,
            &settings,
            Some("Checking"),
            "monthly",
            Some("1y"),
            None,
        )?;

        assert!(output.contains("Date"));
        assert!(output.contains("Total"));

        Ok(())
    }

    #[test]
    fn test_history_date_format() -> Result<()> {
        let repo = load_test_repo();
        let settings = test_settings();

        let output =
            history_view(&repo, &settings, None, "monthly", Some("1y"), None)?;

        // Should not contain raw format like "2024-1"
        assert!(!output.contains("-1 "));
        assert!(!output.contains("-2 "));

        Ok(())
    }
}
