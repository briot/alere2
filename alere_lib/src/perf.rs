use crate::{
    accounts::Account,
    commodities::Commodity,
    market_prices::MarketPrices,
    multi_values::{MultiValue, Operation, Value},
    repositories::Repository,
};
use anyhow::Result;
use chrono::{DateTime, Local};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

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
    pub pnl: MultiValue,
    pub period_pnl: MultiValue,
    pub average_cost: Option<MultiValue>,
    pub weighted_average: Option<MultiValue>,
    pub price: Option<MultiValue>,
}

impl Performance {
    fn calculate_irr(cash_flows: &[(DateTime<Local>, Decimal)], final_value: Decimal, now: DateTime<Local>) -> Option<Decimal> {
        if cash_flows.is_empty() {
            return None;
        }
        
        // Newton-Raphson method to find IRR
        let mut rate = Decimal::from_f64_retain(0.1).unwrap(); // Initial guess: 10%
        let max_iterations = 100;
        let tolerance = Decimal::from_f64_retain(0.0001).unwrap();
        
        for _ in 0..max_iterations {
            let mut npv = Decimal::ZERO;
            let mut npv_derivative = Decimal::ZERO;
            
            for (date, amount) in cash_flows {
                let years = Decimal::from_f64_retain((now - *date).num_days() as f64 / 365.25).unwrap();
                if let (Some(y), Some(r)) = (years.to_f64(), rate.to_f64()) {
                    let discount = (1.0 + r).powf(y.into());
                    let pv = amount.to_f64().unwrap() / discount;
                    npv += Decimal::from_f64_retain(pv).unwrap();
                    npv_derivative -= Decimal::from_f64_retain(pv * y / (1.0 + r)).unwrap();
                }
            }
            
            // Add final value (current equity)
            npv += final_value;
            
            if npv.abs() < tolerance {
                return Some(rate);
            }
            
            if npv_derivative.is_zero() {
                return None;
            }
            
            rate = rate - npv / npv_derivative;
            
            // Prevent unrealistic rates
            if rate < Decimal::from(-1) || rate > Decimal::from(10) {
                return None;
            }
        }
        
        None
    }
    
    fn new(
        account: &Account,
        args: PerfArgs,
        prices: &mut MarketPrices,
        now: DateTime<Local>,
    ) -> Self {
        let equity = if account.get_kind().is_stock() {
            prices.convert_multi_value(&args.shares, &now)
        } else {
            &args.invested + &args.unrealized
        };

        let shares = args.shares.iter().next().map(|v| v.amount);
        let roi = (&equity + &args.realized) / &args.invested;
        
        let annualized_roi = if !args.cash_flows.is_empty() {
            if let Some(final_val) = (&equity + &args.realized).iter().next() {
                Self::calculate_irr(&args.cash_flows, final_val.amount, now)
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
                                        args.cash_flows.push((s.post_ts, -val.amount));
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
                                    let invested_val = prices
                                        .convert_value(amount, &s.post_ts);
                                    let fees = prices
                                        .convert_multi_value(
                                            &external_amount,
                                            &s.post_ts,
                                        );
                                    let net = &invested_val - &fees;
                                    if let Some(val) = net.iter().next() {
                                        args.cash_flows.push((s.post_ts, -val.amount));
                                    }
                                    args.invested += invested_val;
                                    args.invested -= fees;
                                } else {
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
                                    args.cash_flows.push((s.post_ts, -val.amount));
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

            result.push(Performance::new(&acc, args, &mut prices, now));
        }

        Ok(result)
    }
}
