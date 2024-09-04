use alere_lib::{repositories::Repository, stats::Stats};
use rust_decimal::Decimal;

pub struct Settings {}

pub fn stats_view(
    repo: &Repository,
    stats: Stats,
    _settings: Settings,
) -> String {
    let period_mkt_income =  // Sign is negative, so negate the diff
        &stats.start.mkt_all_income - &stats.end.mkt_all_income;
    let period_income =  // Sign is negative, so negate the diff
        &stats.start.all_income - &stats.end.all_income;
    let period_mkt_passive_income =
        &stats.start.mkt_passive_income - &stats.end.mkt_passive_income;
    let period_mkt_unrealized =
        &stats.start.mkt_unrealized - &stats.end.mkt_unrealized;
    let period_unrealized = &stats.start.unrealized - &stats.end.unrealized;
    let period_mkt_expense =
        &stats.end.mkt_all_expense - &stats.start.mkt_all_expense;
    let period_expense = &stats.end.all_expense - &stats.start.all_expense;
    let mkt_cashflow = &period_mkt_income - &period_mkt_expense;

    format!(
        "
Networth:               {} to {}
Income:                 {} ({} to {})
Passive income:            {}
Expenses:               {} ({} to {})
Cashflow:               {}
Unrealized:             {}
Computed delta:         {} (networth delta {})
Computed Market delta:  {} (networth delta {})
Savings Rate:           {:.1?}%
Financial Independence: {:.1?}%
Passive Income:         {:.1?}%
",
        repo.display_multi_value(&stats.start.mkt_networth),
        repo.display_multi_value(&stats.end.mkt_networth),
        repo.display_multi_value(&period_mkt_income),
        repo.display_multi_value(&stats.start.mkt_all_income),
        repo.display_multi_value(&stats.end.mkt_all_income),
        repo.display_multi_value(&period_mkt_passive_income),
        repo.display_multi_value(&period_mkt_expense),
        repo.display_multi_value(&stats.start.mkt_all_expense),
        repo.display_multi_value(&stats.end.mkt_all_expense),
        repo.display_multi_value(&mkt_cashflow),
        repo.display_multi_value(&period_mkt_unrealized),
        repo.display_multi_value(
            &(&period_income - &period_expense + &period_unrealized)
        ),
        repo.display_multi_value(
            &(&stats.end.networth - &stats.start.networth)
        ),
        repo.display_multi_value(
            &(&period_mkt_income - &period_mkt_expense
                + &period_mkt_unrealized)
        ),
        repo.display_multi_value(
            &(&stats.end.mkt_networth - &stats.start.mkt_networth)
        ),
        (&mkt_cashflow / &period_mkt_income).map(|p| p * Decimal::ONE_HUNDRED),
        (&(&period_mkt_passive_income + &period_mkt_unrealized)
            / &period_mkt_expense)
            .map(|p| p * Decimal::ONE_HUNDRED),
        (&(&period_mkt_passive_income + &period_mkt_unrealized)
            / &period_mkt_income)
            .map(|p| p * Decimal::ONE_HUNDRED),
    )
}
