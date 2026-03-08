use alere_lib::repositories::Repository;
use anyhow::Result;
use rust_decimal::prelude::ToPrimitive;
use stock_importer::{StockSource, YahooFinance};

use crate::global_settings::GlobalSettings;

pub async fn update_prices(repo: &mut Repository, settings: &GlobalSettings) -> Result<()> {
    // Show current networth
    println!("Current Networth:");
    let periods = vec![alere_lib::times::Intv::UpTo(alere_lib::times::Instant::Now)];
    crate::networth(repo, settings, periods, false, false, false, false, false, false, false, false)?;
    println!();

    // Collect all stocks with quote symbols
    let source = YahooFinance::new()?;
    let stocks: Vec<_> = repo.commodities.iter_commodities()
        .filter(|c| !c.is_currency())
        .collect();

    if stocks.is_empty() {
        println!("No stocks to display.");
        return Ok(());
    }

    println!("Stocks:");
    for commodity in &stocks {
        let name = commodity.get_name();
        let quote_symbol = commodity.get_quote_symbol();
        let quote_source = commodity.get_quote_source();
        println!("  {} - symbol: {:?}, source: {:?}", 
            name, 
            quote_symbol.as_deref().unwrap_or("None"),
            quote_source.as_deref().unwrap_or("None")
        );
    }
    println!();

    // Only fetch stocks with both quote_symbol and quote_source
    let fetchable: Vec<_> = stocks.iter()
        .filter_map(|c| {
            if let (Some(symbol), Some(_source)) = (c.get_quote_symbol(), c.get_quote_source()) {
                Some(((*c).clone(), symbol))
            } else {
                None
            }
        })
        .collect();

    if fetchable.is_empty() {
        println!("No stocks with quote_symbol and quote_source to fetch.");
        return Ok(());
    }

    println!("Fetching {} stock prices...\n", fetchable.len());

    // Debug: fetch first stock only and show response
    if let Some((commodity, symbol)) = fetchable.first() {
        println!("Fetching {}...", symbol);
        
        match source.fetch_price(symbol).await {
            Ok(price_data) => {
                println!("\n=== Success ===");
                println!("Symbol: {}", price_data.symbol);
                println!("Price: {:.2}", price_data.price);
                println!("Timestamp: {}", price_data.timestamp);
                
                let mut prices = repo.market_prices(settings.commodity.clone());
                let old_price_opt = prices.get_price(commodity, &settings.reftime);
                
                if let Some(old) = old_price_opt {
                    if let Some(old_f64) = old.to_f64() {
                        println!("Old price: {:.2}", old_f64);
                        let diff = (price_data.price - old_f64) / old_f64 * 100.0;
                        println!("Change: {:+.2}%", diff);
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    Ok(())
}
