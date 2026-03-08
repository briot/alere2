use alere_lib::repositories::Repository;
use anyhow::Result;
use futures::future::join_all;
use rust_decimal::prelude::ToPrimitive;
use stock_importer::{StockSource, YahooFinance};
use tabled::builder::Builder;

use crate::global_settings::GlobalSettings;

type PriceUpdate = (alere_lib::commodities::Commodity, rust_decimal::Decimal, chrono::DateTime<chrono::Local>);

fn show_current_networth(
    repo: &mut Repository,
    settings: &GlobalSettings,
) -> Result<()> {
    let periods =
        vec![alere_lib::times::Intv::UpTo(alere_lib::times::Instant::Now)];
    crate::networth(
        repo, settings, periods, false, false, false, false, false, false,
        false, false,
    )
}

pub async fn update_prices(
    repo: &mut Repository,
    settings: &GlobalSettings,
) -> Result<()> {
    println!("Current Networth:");
    show_current_networth(repo, settings)?;
    println!();

    let source = YahooFinance::new()?;
    #[allow(clippy::mutable_key_type)]
    let commodity_shares = repo.compute_commodity_balances();

    let mut fetchable: Vec<_> = repo
        .commodities
        .iter_commodities()
        .filter(|c| !c.is_currency())
        .filter_map(|c| {
            if let (Some(symbol), Some(_source)) =
                (c.get_quote_symbol(), c.get_quote_source())
            {
                let shares = commodity_shares
                    .get(c)
                    .copied()
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                if shares > rust_decimal::Decimal::ZERO {
                    Some((c.clone(), symbol, shares))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if fetchable.is_empty() {
        println!(
            "No stocks with quote_symbol, quote_source, and non-zero shares to fetch."
        );
        return Ok(());
    }

    fetchable.sort_by(|a, b| a.0.get_name().cmp(&b.0.get_name()));
    println!("Fetching {} stock prices...\n", fetchable.len());

    let all_results = fetch_prices_in_batches(&source, &fetchable).await;
    display_and_update_prices(repo, settings, fetchable, all_results)?;

    println!("\nUpdated Networth:");
    show_current_networth(repo, settings)?;

    Ok(())
}

async fn fetch_prices_in_batches(
    source: &YahooFinance,
    fetchable: &[(
        alere_lib::commodities::Commodity,
        String,
        rust_decimal::Decimal,
    )],
) -> Vec<Result<stock_importer::StockPrice, anyhow::Error>> {
    let batch_size = 10;
    let mut all_results = Vec::new();

    for (i, chunk) in fetchable.chunks(batch_size).enumerate() {
        let fetch_tasks: Vec<_> = chunk
            .iter()
            .map(|(_, symbol, _)| source.fetch_price(symbol))
            .collect();

        let results = join_all(fetch_tasks).await;
        all_results.extend(results);

        if i < fetchable.len().div_ceil(batch_size) - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    all_results
}

fn display_and_update_prices(
    repo: &mut Repository,
    settings: &GlobalSettings,
    fetchable: Vec<(
        alere_lib::commodities::Commodity,
        String,
        rust_decimal::Decimal,
    )>,
    all_results: Vec<Result<stock_importer::StockPrice, anyhow::Error>>,
) -> Result<()> {
    let mut builder = Builder::default();
    builder.push_record([
        "Name",
        "Symbol",
        "Shares",
        "Old Price",
        "Old Date",
        "New Price",
        "New Date",
        "Change",
    ]);

    let mut prices = repo.market_prices(settings.commodity.clone());
    let mut new_prices = Vec::new();

    for ((commodity, symbol, shares), result) in
        fetchable.iter().zip(all_results.iter())
    {
        if let Ok(price_data) = result {
            if let Some((row, price_update)) = build_price_row(
                commodity,
                symbol,
                shares,
                price_data,
                &mut prices,
                settings,
            ) {
                builder.push_record(row);
                new_prices.push(price_update);
            }
        } else if let Err(e) = result {
            eprintln!("Failed to fetch {}: {}", symbol, e);
        }
    }

    let mut table = builder.build();
    settings.style.apply(&mut table);
    println!("\nStock Price Updates:");
    println!("{}", table);

    update_prices_in_repo(repo, settings, new_prices);

    Ok(())
}

fn build_price_row(
    commodity: &alere_lib::commodities::Commodity,
    symbol: &str,
    shares: &rust_decimal::Decimal,
    price_data: &stock_importer::StockPrice,
    prices: &mut alere_lib::market_prices::MarketPrices,
    settings: &GlobalSettings,
) -> Option<([String; 8], PriceUpdate)> {
    let old_price_opt =
        prices.get_price_with_date(commodity, &settings.reftime);

    let should_update = old_price_opt
        .as_ref()
        .map(|old| price_data.timestamp > old.timestamp)
        .unwrap_or(true);

    if !should_update {
        return None;
    }

    let (old_price_str, old_date_str) = old_price_opt
        .as_ref()
        .map(|p| {
            (
                format!("{:.2}", p.price.to_f64().unwrap_or(0.0)),
                p.timestamp.format("%Y-%m-%d").to_string(),
            )
        })
        .unwrap_or_else(|| ("N/A".to_string(), "N/A".to_string()));

    let new_price = format!("{:.2}", price_data.price);
    let new_date = price_data.timestamp.format("%Y-%m-%d").to_string();
    let shares_str = format!("{:.2}", shares.to_f64().unwrap_or(0.0));

    let change = if let Some(ref old) = old_price_opt {
        let old_f64 = old.price.to_f64().unwrap_or(0.0);
        if old_f64 > 0.0 {
            let diff = (price_data.price - old_f64) / old_f64 * 100.0;
            format!("{:+.2}%", diff)
        } else {
            "N/A".to_string()
        }
    } else {
        "N/A".to_string()
    };

    let row = [
        commodity.get_name().to_string(),
        symbol.to_string(),
        shares_str,
        old_price_str,
        old_date_str,
        new_price,
        new_date,
        change,
    ];

    let price_update = (
        commodity.clone(),
        rust_decimal::Decimal::from_f64_retain(price_data.price)
            .unwrap_or_default(),
        price_data.timestamp,
    );

    Some((row, price_update))
}

fn update_prices_in_repo(
    repo: &mut Repository,
    settings: &GlobalSettings,
    new_prices: Vec<PriceUpdate>,
) {
    if let Some(ref target_commodity) = settings.commodity {
        for (commodity, price_value, timestamp) in new_prices {
            let price = alere_lib::prices::Price::new(
                timestamp,
                price_value,
                alere_lib::price_sources::PriceSourceFrom::Transaction,
            );
            repo.add_price(&commodity, target_commodity, price);
        }
    }
}
