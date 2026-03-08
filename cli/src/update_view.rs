use alere_lib::repositories::Repository;
use anyhow::Result;
use futures::future::join_all;
use rust_decimal::prelude::ToPrimitive;
use stock_importer::{StockSource, YahooFinance};
use tabled::builder::Builder;

use crate::global_settings::GlobalSettings;

pub async fn update_prices(repo: &mut Repository, settings: &GlobalSettings) -> Result<()> {
    // Show current networth
    println!("Current Networth:");
    let periods = vec![alere_lib::times::Intv::UpTo(alere_lib::times::Instant::Now)];
    crate::networth(repo, settings, periods, false, false, false, false, false, false, false, false)?;
    println!();

    // Collect stocks with both quote_symbol and quote_source, and non-zero shares
    let source = YahooFinance::new()?;
    
    // Calculate shares for each commodity
    let mut commodity_shares = std::collections::HashMap::new();
    for account in repo.accounts().iter() {
        account.for_each_split(|split| {
            match &split.operation {
                alere_lib::multi_values::Operation::Credit(mv) => {
                    for value in mv.iter() {
                        *commodity_shares.entry(value.commodity.clone()).or_insert(rust_decimal::Decimal::ZERO) += value.amount;
                    }
                }
                alere_lib::multi_values::Operation::BuyAmount { qty, .. } 
                | alere_lib::multi_values::Operation::BuyPrice { qty, .. } 
                | alere_lib::multi_values::Operation::AddShares { qty } => {
                    *commodity_shares.entry(qty.commodity.clone()).or_insert(rust_decimal::Decimal::ZERO) += qty.amount;
                }
                alere_lib::multi_values::Operation::Reinvest { shares, .. } => {
                    for value in shares.iter() {
                        *commodity_shares.entry(value.commodity.clone()).or_insert(rust_decimal::Decimal::ZERO) += value.amount;
                    }
                }
                _ => {}
            }
        });
    }
    
    let mut fetchable: Vec<_> = repo.commodities.iter_commodities()
        .filter(|c| !c.is_currency())
        .filter_map(|c| {
            if let (Some(symbol), Some(_source)) = (c.get_quote_symbol(), c.get_quote_source()) {
                let shares = commodity_shares.get(c).copied().unwrap_or(rust_decimal::Decimal::ZERO);
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
        println!("No stocks with quote_symbol, quote_source, and non-zero shares to fetch.");
        return Ok(());
    }

    // Sort by commodity name
    fetchable.sort_by(|a, b| a.0.get_name().cmp(&b.0.get_name()));

    println!("Fetching {} stock prices...\n", fetchable.len());

    // Fetch in batches of 10
    let batch_size = 10;
    let mut all_results = Vec::new();
    
    for (i, chunk) in fetchable.chunks(batch_size).enumerate() {
        let fetch_tasks: Vec<_> = chunk.iter()
            .map(|(_, symbol, _)| source.fetch_price(symbol))
            .collect();
        
        let results = join_all(fetch_tasks).await;
        all_results.extend(results);
        
        // Small delay between batches
        if i < (fetchable.len() + batch_size - 1) / batch_size - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    // Build results table and collect new prices
    let mut builder = Builder::default();
    builder.push_record(["Name", "Symbol", "Shares", "Old Price", "Old Date", "New Price", "New Date", "Change"]);

    let mut prices = repo.market_prices(settings.commodity.clone());
    let now = chrono::Local::now();
    let mut new_prices = Vec::new();
    
    for ((commodity, symbol, shares), result) in fetchable.iter().zip(all_results.iter()) {
        match result {
            Ok(price_data) => {
                let old_price_opt = prices.get_price_with_date(commodity, &settings.reftime);
                
                // Only update if price is newer than existing
                let should_update = old_price_opt
                    .as_ref()
                    .map(|old| price_data.timestamp > old.timestamp)
                    .unwrap_or(true);
                
                if !should_update {
                    continue;
                }
                
                let (old_price_str, old_date_str) = old_price_opt
                    .as_ref()
                    .map(|p| (
                        format!("{:.2}", p.price.to_f64().unwrap_or(0.0)),
                        p.timestamp.format("%Y-%m-%d").to_string()
                    ))
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
                
                builder.push_record([
                    commodity.get_name().as_ref(),
                    symbol.as_str(),
                    shares_str.as_str(),
                    old_price_str.as_str(),
                    old_date_str.as_str(),
                    new_price.as_str(),
                    new_date.as_str(),
                    change.as_str(),
                ]);

                new_prices.push((
                    commodity.clone(),
                    rust_decimal::Decimal::from_f64_retain(price_data.price).unwrap_or_default(),
                    price_data.timestamp,
                ));
            }
            Err(e) => {
                eprintln!("Failed to fetch {}: {}", symbol, e);
            }
        }
    }

    let mut table = builder.build();
    settings.style.apply(&mut table);
    println!("\nStock Price Updates:");
    println!("{}", table);

    // Update prices in repository
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

    // Recompute networth with new prices
    println!("\nUpdated Networth:");
    let periods = vec![alere_lib::times::Intv::UpTo(alere_lib::times::Instant::Now)];
    crate::networth(repo, settings, periods, false, false, false, false, false, false, false, false)?;

    Ok(())
}
