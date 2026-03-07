use crate::{
    accounts::Account,
    commodities::Commodity,
    market_prices::MarketPrices,
    multi_values::{MultiValue, Operation, Value},
    repositories::Repository,
};
use anyhow::Result;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

pub struct Settings {
    pub commodity: Option<Commodity>,
}

#[derive(Default)]
struct PerfArgs {
    shares: MultiValue,
    invested: MultiValue,
    realized: MultiValue,

    // Equity can be computed two ways: if we have shares, this will be the
    // total number of shares times the price.  But users might also simply
    // track with some "unrealized" credits.
    unrealized: MultiValue,

    first_tx: Option<DateTime<Local>>,
    cash_flows: Vec<(DateTime<Local>, Decimal)>,
}

pub struct Performance {
    pub account: Account, //  The symbol
    pub shares: MultiValue,
    pub invested: MultiValue,
    pub realized: MultiValue,
    pub equity: MultiValue,
    pub roi: Option<Decimal>,
    pub period_roi: Option<Decimal>,
    pub annualized_roi: Option<Decimal>,
    pub irr: Option<Decimal>,
    pub pnl: MultiValue,
    pub period_pnl: MultiValue,
    pub average_cost: Option<MultiValue>,
    pub weighted_average: Option<MultiValue>,
    pub price: Option<MultiValue>,
}

impl Performance {
    // Internal Rate of Return (IRR) calculation using Newton-Raphson method
    //
    // IRR is the discount rate that makes the Net Present Value (NPV) of all cash flows equal to zero.
    // Formula: NPV = Σ(CF_i / (1+r)^t_i) + FV = 0
    // where CF_i are cash flows, r is the rate, t_i is time in years, and FV is final value
    //
    // Newton-Raphson iteratively solves: r_new = r_old - NPV / NPV'
    // where NPV' is the derivative of NPV with respect to r
    fn calculate_irr(
        cash_flows: &[(DateTime<Local>, Decimal)],
        final_value: Decimal,
        now: DateTime<Local>,
    ) -> Option<Decimal> {
        if cash_flows.is_empty() {
            return None;
        }

        // Calculate simple ROI as initial guess
        let total_invested: Decimal =
            cash_flows.iter().map(|(_, amt)| -amt).sum();
        let simple_roi = if !total_invested.is_zero() {
            (final_value / total_invested - Decimal::ONE)
                .to_f64()
                .unwrap_or(0.1)
        } else {
            0.1
        };

        // Try multiple initial guesses, starting with simple ROI
        for initial_rate in [simple_roi, 0.05, 0.10, 0.01, -0.05] {
            if let Some(irr) = Self::try_irr_with_guess(
                cash_flows,
                final_value,
                now,
                initial_rate,
            ) {
                // Verify the result is reasonable
                if irr > Decimal::from_f64_retain(-0.99).unwrap()
                    && irr < Decimal::from(10)
                {
                    return Some(irr);
                }
            }
        }
        None
    }

    // Try to find IRR with a specific initial guess
    // Returns None if convergence fails or result is unrealistic
    fn try_irr_with_guess(
        cash_flows: &[(DateTime<Local>, Decimal)],
        final_value: Decimal,
        now: DateTime<Local>,
        initial_guess: f64,
    ) -> Option<Decimal> {
        // Newton-Raphson method to find IRR
        let mut rate = Decimal::from_f64_retain(initial_guess).unwrap();
        let max_iterations = 100;
        let tolerance = Decimal::from_f64_retain(0.0001).unwrap();

        for _ in 0..max_iterations {
            let mut npv = Decimal::ZERO;
            let mut npv_derivative = Decimal::ZERO;

            let Some(r) = rate.to_f64() else {
                return None;
            };

            if r <= -0.99 {
                return None;
            }

            for (date, amount) in cash_flows {
                let years = (now - *date).num_days() as f64 / 365.25;

                let compound = (1.0 + r).powf(years);
                if !compound.is_finite() || compound == 0.0 {
                    return None;
                }

                let Some(amt) = amount.to_f64() else {
                    continue;
                };
                // Compound the past cash flow forward to now
                let fv = amt * compound;

                let Some(fv_dec) = Decimal::from_f64_retain(fv) else {
                    return None;
                };
                npv += fv_dec;

                // Derivative: d/dr[amt * (1+r)^t] = amt * t * (1+r)^(t-1)
                let deriv_val = fv * years / (1.0 + r);
                let Some(deriv_dec) = Decimal::from_f64_retain(deriv_val)
                else {
                    return None;
                };
                npv_derivative += deriv_dec;
            }

            // Add final value (current equity) as positive cash flow at now
            npv += final_value;

            if npv.abs() < tolerance {
                return Some(rate);
            }

            if npv_derivative.is_zero() {
                return None;
            }

            rate -= npv / npv_derivative;

            // Prevent unrealistic rates
            if rate < Decimal::from_f64_retain(-0.99).unwrap()
                || rate > Decimal::from(10)
            {
                return None;
            }
        }

        None
    }

    fn new(
        account: &Account,
        args: PerfArgs,
        prices: &mut MarketPrices,
        target_commodity: Option<&Commodity>,
        now: DateTime<Local>,
    ) -> Self {
        // For closed accounts, use closed date instead of now
        let end_date = account.get_closed_date().unwrap_or(now);

        let equity = if account.get_kind().is_stock() {
            prices.convert_multi_value(&args.shares, &end_date)
        } else {
            &args.invested + &args.unrealized
        };

        let shares = args.shares.iter().next().map(|v| v.amount);
        let roi = (&equity + &args.realized) / &args.invested;

        let annualized_roi =
            if let (Some(r), Some(first)) = (roi, args.first_tx) {
                let years = (end_date - first).num_days() as f64 / 365.25;
                if years > 0.0 {
                    Some(
                        Decimal::from_f64_retain(
                            r.to_f64().unwrap().powf(1.0 / years),
                        )
                        .unwrap(),
                    )
                } else {
                    None
                }
            } else {
                None
            };

        // Calculate IRR using cash flows and current equity
        // IRR requires all values in the same currency
        let irr = if !args.cash_flows.is_empty() {
            if let (Some(equity_val), Some(target)) = (equity.iter().next(), target_commodity) {
                // Check if equity is in the target currency (not shares)
                if equity_val.commodity == *target {
                    Self::calculate_irr(&args.cash_flows, equity_val.amount, end_date)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Performance {
            account: account.clone(),
            roi,
            period_roi: None,
            annualized_roi,
            irr,
            pnl: &equity - &args.invested + &args.realized,
            period_pnl: MultiValue::default(),
            average_cost: shares.map(|s| (&args.invested - &args.realized) / s),
            weighted_average: shares.map(|s| &args.invested / s),
            price: args.shares.commodity().map(|c| {
                prices.convert_multi_value(
                    &MultiValue::new(Decimal::ONE, &c),
                    &now,
                )
            }),
            equity,
            shares: args.shares,
            invested: args.invested,
            realized: args.realized,
        }
    }

    pub fn load(
        repo: &Repository,
        settings: Settings,
        now: DateTime<Local>,
    ) -> Result<Vec<Self>> {
        let mut result = Vec::new();
        let mut prices = repo.market_prices(settings.commodity.clone());

        for acc in repo.accounts.iter() {
            if !acc.get_kind().is_trading() {
                continue;
            }

            let mut args = PerfArgs::default();

            // All the user's money that went into this operation.  For
            // instance, when we buy shares, we would typically have the
            // following splits:
            //    Checking account (-1000)
            //    Investment account (+900)   // current acc
            //    Fees (+100)                 // not a user account
            // The external amount would be +100 in this case, ignoring both
            // acc and any split applying to a user account.  This is how
            // much the operation actually cost.

            for tx in acc.iter_transactions() {
                if args.first_tx.is_none() {
                    args.first_tx = tx.splits().first().map(|s| s.post_ts);
                }

                let mut external_amount = MultiValue::zero();
                let mut internal_unrealized = MultiValue::zero();
                let mut is_unrealized = false;
                for s in tx.splits().iter() {
                    if s.account != acc {
                        match &s.operation {
                            Operation::Credit(v) => {
                                if s.account.get_kind().is_unrealized() {
                                    is_unrealized = true;
                                    internal_unrealized += v;
                                } else if !s.account.get_kind().is_user() {
                                    external_amount -= v;
                                }
                            }
                            Operation::BuyAmount { qty, .. } => {
                                // Used for dividends in foreign currencies.
                                // E.g, the stock is in $ and the splits are:
                                //   * taxes: qty=5.11EUR  amount=$6.01
                                //   * dividend: -11.38EUR for $-13.39
                                if s.account.get_kind().is_unrealized() {
                                    is_unrealized = true;
                                    internal_unrealized += qty;
                                } else if !s.account.get_kind().is_user() {
                                    external_amount -= qty;
                                }
                            }
                            Operation::AddShares { .. }
                            | Operation::BuyPrice { .. }
                            | Operation::Reinvest { .. }
                            | Operation::Split { .. }
                            | Operation::Dividend => {}
                        }
                    }
                }

                for s in tx.splits().iter() {
                    if s.account == acc {
                        match &s.operation {
                            Operation::Credit(v) => {
                                let v2 =
                                    prices.convert_multi_value(v, &s.post_ts);
                                if is_unrealized {
                                    args.unrealized += &v2;
                                } else {
                                    if let Some(val) = v2.iter().next() {
                                        args.cash_flows
                                            .push((s.post_ts, -val.amount));
                                    }
                                    args.invested += v2;
                                }
                            }
                            Operation::AddShares { qty } => {
                                args.shares += qty;
                            }
                            Operation::BuyAmount { qty, amount } => {
                                args.shares += qty;

                                if !qty.is_negative() {
                                    // Buying shares
                                    let invested_val = prices
                                        .convert_value(amount, &s.post_ts);
                                    let fees = prices.convert_multi_value(
                                        &external_amount,
                                        &s.post_ts,
                                    );
                                    let net = &invested_val - &fees;
                                    if let Some(val) = net.iter().next() {
                                        args.cash_flows
                                            .push((s.post_ts, -val.amount));
                                    }
                                    args.invested += invested_val;
                                    args.invested -= fees;
                                } else {
                                    // Selling shares
                                    // Compute sale proceeds from transaction balance
                                    // Transaction is balanced, so sum of all splits = 0
                                    // Sale proceeds = -(sum of non-investment account splits)
                                    let mut sale_proceeds = MultiValue::zero();
                                    for split in tx.splits().iter() {
                                        if split.account != acc {
                                            match &split.operation {
                                                Operation::Credit(v) => {
                                                    sale_proceeds += v;
                                                }
                                                Operation::BuyAmount { qty: q, .. } => {
                                                    sale_proceeds += &MultiValue::new(q.amount, &q.commodity);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    
                                    let proceeds = prices.convert_multi_value(&sale_proceeds, &s.post_ts);
                                    if let Some(val) = proceeds.iter().next() {
                                        args.cash_flows.push((s.post_ts, val.amount));
                                    }
                                    
                                    args.realized -= prices
                                        .convert_value(amount, &s.post_ts);
                                    args.realized += prices
                                        .convert_multi_value(
                                            &external_amount,
                                            &s.post_ts,
                                        );
                                }
                            }
                            Operation::BuyPrice { qty, price } => {
                                args.shares += qty;
                                let fees = prices.convert_multi_value(
                                    &external_amount,
                                    &s.post_ts,
                                );
                                let invested_val = prices.convert_value(
                                    &Value {
                                        commodity: price.commodity.clone(),
                                        amount: qty.amount * price.amount,
                                    },
                                    &s.post_ts,
                                );
                                let net = &invested_val - &fees;
                                if let Some(val) = net.iter().next() {
                                    args.cash_flows
                                        .push((s.post_ts, -val.amount));
                                }
                                args.invested -= fees;
                                args.invested += invested_val;
                            }
                            Operation::Reinvest { .. } => {}
                            Operation::Split { ratio, commodity } => {
                                args.shares.split(commodity, *ratio);
                            }
                            Operation::Dividend => {
                                //  Also count internal_unrealized in case the
                                //  dividend was wrongly classified by user.
                                args.realized += prices.convert_multi_value(
                                    &external_amount,
                                    &s.post_ts,
                                );
                                args.realized -= prices.convert_multi_value(
                                    &internal_unrealized,
                                    &s.post_ts,
                                );
                            }
                        };
                    }
                }
                //println!(
                //    "MANU {} shares={} invested={} realized={}",
                //    tx.display(&Formatter::default()),
                //    args.shares.display(&Formatter::default()),
                //    args.invested.display(&Formatter::default()),
                //    args.realized.display(&Formatter::default())
                //);
                //dbg!(tx, &args.shares, &args.invested, &args.realized);
            }

            result.push(Performance::new(&acc, args, &mut prices, settings.commodity.as_ref(), now));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_irr_simple_investment() {
        // Invest $1000, get back $1100 after 1 year
        // Expected IRR: 10%
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![(
            Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Decimal::from(-1000),
        )];
        let final_value = Decimal::from(1100);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            (irr_val - 0.10).abs() < 0.01,
            "Expected ~10%, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_multiple_investments() {
        // Invest $1000 at start, $500 after 6 months, get back $1650 after 1 year
        // Expected IRR: ~8-9%
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![
            (
                Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Decimal::from(-1000),
            ),
            (
                Local.with_ymd_and_hms(2024, 7, 1, 0, 0, 0).unwrap(),
                Decimal::from(-500),
            ),
        ];
        let final_value = Decimal::from(1650);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            irr_val > 0.0 && irr_val < 0.15,
            "Expected positive IRR ~8-9%, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_loss() {
        // Invest $1000, get back $900 after 1 year
        // Expected IRR: -10%
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![(
            Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Decimal::from(-1000),
        )];
        let final_value = Decimal::from(900);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            (irr_val + 0.10).abs() < 0.01,
            "Expected ~-10%, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_long_term() {
        // Invest $1000, get back $2000 after 10 years
        // Expected IRR: ~7.2% (rule of 72: 72/10 = 7.2)
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![(
            Local.with_ymd_and_hms(2015, 1, 1, 0, 0, 0).unwrap(),
            Decimal::from(-1000),
        )];
        let final_value = Decimal::from(2000);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            (irr_val - 0.072).abs() < 0.01,
            "Expected ~7.2%, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_with_sale() {
        // Invest $1000, sell for $1100 after 1 year (all shares sold, final value = 0)
        // Expected IRR: 10%
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![
            (
                Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Decimal::from(-1000),  // Investment
            ),
            (
                Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
                Decimal::from(1100),  // Sale proceeds
            ),
        ];
        let final_value = Decimal::ZERO;  // All shares sold

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            (irr_val - 0.10).abs() < 0.01,
            "Expected ~10%, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_partial_sale() {
        // Invest $1000, sell half for $600 after 6 months, remaining equity $500
        // Total value: $600 + $500 = $1100, so 10% return over 1 year
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![
            (
                Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Decimal::from(-1000),
            ),
            (
                Local.with_ymd_and_hms(2024, 7, 1, 0, 0, 0).unwrap(),
                Decimal::from(600),
            ),
        ];
        let final_value = Decimal::from(500);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            irr_val > 0.0 && irr_val < 0.20,
            "Expected positive IRR, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_no_cash_flows() {
        // No cash flows - should return None
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![];
        let final_value = Decimal::from(1000);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_none());
    }

    #[test]
    fn test_irr_zero_final_value_with_loss() {
        // Invest $1000, sell for $500 (50% loss), all shares sold
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![
            (
                Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Decimal::from(-1000),
            ),
            (
                Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
                Decimal::from(500),
            ),
        ];
        let final_value = Decimal::ZERO;

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            irr_val < 0.0 && irr_val > -0.60,
            "Expected negative IRR ~-50%, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_multiple_buys_and_sells() {
        // Buy $1000, buy $500, sell $800, remaining equity $800
        // Total invested: $1500, total value: $800 + $800 = $1600
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![
            (
                Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Decimal::from(-1000),
            ),
            (
                Local.with_ymd_and_hms(2024, 4, 1, 0, 0, 0).unwrap(),
                Decimal::from(-500),
            ),
            (
                Local.with_ymd_and_hms(2024, 10, 1, 0, 0, 0).unwrap(),
                Decimal::from(800),
            ),
        ];
        let final_value = Decimal::from(800);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            irr_val > 0.0,
            "Expected positive IRR, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_extreme_loss() {
        // Invest $1000, lose almost everything (99% loss)
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![(
            Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Decimal::from(-1000),
        )];
        let final_value = Decimal::from(10);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap().to_f64().unwrap();
        assert!(
            irr_val < -0.90,
            "Expected very negative IRR ~-99%, got {}",
            irr_val
        );
    }

    #[test]
    fn test_irr_formatting_bug() {
        // Test that IRR is formatted correctly (not as ROI)
        // IRR of 10% should display as "10.00%", not "9.00%" (which would be (1.10 - 1) * 100)
        let now = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let cash_flows = vec![(
            Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Decimal::from(-1000),
        )];
        let final_value = Decimal::from(1100);

        let irr = Performance::calculate_irr(&cash_flows, final_value, now);
        assert!(irr.is_some());
        let irr_val = irr.unwrap();
        
        // IRR should be around 0.10 (10%), not 1.10
        assert!(
            irr_val > Decimal::from_f64_retain(0.09).unwrap() && 
            irr_val < Decimal::from_f64_retain(0.11).unwrap(),
            "Expected IRR around 0.10, got {}",
            irr_val
        );
    }
}
