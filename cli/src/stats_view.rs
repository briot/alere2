use alere_lib::{repositories::Repository, stats::Stats};
use rust_decimal::Decimal;

pub struct Settings {}

pub fn stats_view(
    repo: &Repository,
    stats: Stats,
    _settings: Settings,
) -> String {
    let abs_income = -&stats.mkt_income;
    let abs_passive_income = -&stats.mkt_passive_income;
    let neg_expense = -&stats.mkt_expense;
    let mkt_cashflow = &abs_income + &neg_expense;

    format!(
        "
Networth:               {} to {}
Income:                 {} = passive {} + ...
Expenses:               {}
Cashflow:               {}
Unrealized:             + {}
Computed Market delta:  {} (networth delta {})
Savings Rate:           {:.1?}%
Financial Independence: {:.1?}%
Passive Income:         {:.1?}%
",
        repo.display_multi_value(&stats.mkt_start_networth),
        repo.display_multi_value(&stats.mkt_end_networth),
        repo.display_multi_value(&abs_income),
        repo.display_multi_value(&abs_passive_income),
        repo.display_multi_value(&neg_expense),
        repo.display_multi_value(&mkt_cashflow),
        repo.display_multi_value(&stats.mkt_unrealized),
        repo.display_multi_value(&(&mkt_cashflow + &stats.mkt_unrealized)),
        repo.display_multi_value(
            &(&stats.mkt_end_networth - &stats.mkt_start_networth)
        ),
        (&mkt_cashflow / &abs_income).map(|p| p * Decimal::ONE_HUNDRED),
        (&(&abs_passive_income + &stats.mkt_unrealized)
            / &stats.mkt_expense)
            .map(|p| p * Decimal::ONE_HUNDRED),
        (&(&abs_passive_income + &stats.mkt_unrealized)
            / &abs_income)
            .map(|p| p * Decimal::ONE_HUNDRED),
    )
}
