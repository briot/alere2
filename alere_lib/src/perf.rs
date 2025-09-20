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

        Performance {
            account: account.clone(),
            roi,
            period_roi: None,
            annualized_roi: None,
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
                                    args.unrealized += v2;
                                } else {
                                    args.invested += v2;
                                }
                            }
                            Operation::AddShares { qty } => {
                                args.shares += qty;
                            }
                            Operation::BuyAmount { qty, amount } => {
                                args.shares += qty;

                                if !qty.is_negative() {
                                    args.invested += prices
                                        .convert_value(amount, &s.post_ts);
                                    args.invested -= prices
                                        .convert_multi_value(
                                            &external_amount,
                                            &s.post_ts,
                                        );
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
                                args.invested -= prices.convert_multi_value(
                                    &external_amount,
                                    &s.post_ts,
                                );
                                args.invested += prices.convert_value(
                                    &Value {
                                        commodity: price.commodity.clone(),
                                        amount: qty.amount * price.amount,
                                    },
                                    &s.post_ts,
                                );
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
