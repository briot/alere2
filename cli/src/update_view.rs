use alere_lib::repositories::Repository;
use anyhow::Result;
use stock_importer::YahooFinance;

use crate::global_settings::GlobalSettings;

pub async fn update_prices(repo: &mut Repository, settings: &GlobalSettings) -> Result<()> {
    // Show current networth
    println!("Current Networth:");
    let periods = vec![alere_lib::times::Intv::UpTo(alere_lib::times::Instant::Now)];
    crate::networth(repo, settings, periods, false, false, false, false, false, false, false, false)?;
    println!();

    // Collect all stocks with quote symbols
    let _source = YahooFinance::new()?;
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
    if let Some((_commodity, symbol)) = fetchable.first() {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()?;
        
        // First visit Yahoo Finance homepage to get cookies
        println!("Getting cookies from Yahoo Finance...");
        let _ = client.get("https://finance.yahoo.com/")
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send().await;
        
        let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{}", symbol);
        println!("Fetching {}...", symbol);
        println!("URL: {}\n", url);
        
        match client.get(&url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept", "application/json")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Referer", "https://finance.yahoo.com/")
            .send().await {
            Ok(resp) => {
                println!("Status: {}", resp.status());
                match resp.text().await {
                    Ok(text) => {
                        println!("\n=== Full Response ===");
                        println!("{}", text);
                        println!("\n=== End Response (length: {} bytes) ===", text.len());
                    }
                    Err(e) => println!("Error reading response: {}", e),
                }
            }
            Err(e) => println!("Error fetching: {}", e),
        }
    }

    Ok(())
}
