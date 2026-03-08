use anyhow::Result;
use chrono::{DateTime, Local, TimeZone};
use regex::Regex;

/// Stock price data
#[derive(Debug, Clone)]
pub struct StockPrice {
    pub symbol: String,
    pub price: f64,
    pub timestamp: DateTime<Local>,
}

/// Trait for stock data sources
#[allow(async_fn_in_trait)]
pub trait StockSource {
    async fn fetch_price(&self, symbol: &str) -> Result<StockPrice>;
}

/// Yahoo Finance data source
pub struct YahooFinance {
    price_regex: Regex,
    time_regex: Regex,
    client: reqwest::Client,
}

impl YahooFinance {
    pub fn new() -> Result<Self> {
        Ok(Self {
            price_regex: Regex::new(r#""regularMarketPrice":([0-9.]+)"#)?,
            time_regex: Regex::new(r#""regularMarketTime":([0-9]+)"#)?,
            client: reqwest::Client::builder()
                .cookie_store(true)
                .build()?,
        })
    }

    pub async fn fetch_price_debug(&self, symbol: &str) -> Result<(String, StockPrice)> {
        let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{}", symbol);
        let response = self.client.get(&url).send().await?.text().await?;
        
        let price = self.price_regex
            .captures(&response)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract price for {}", symbol))?;
        
        let timestamp = self.time_regex
            .captures(&response)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .and_then(|ts| Local.timestamp_opt(ts, 0).single())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract timestamp for {}", symbol))?;
        
        Ok((response, StockPrice {
            symbol: symbol.to_string(),
            price,
            timestamp,
        }))
    }
}

impl StockSource for YahooFinance {
    async fn fetch_price(&self, symbol: &str) -> Result<StockPrice> {
        let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{}", symbol);
        
        let mut retries = 0;
        let response = loop {
            match self.client.get(&url)
                .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .header("Accept", "application/json")
                .header("Accept-Language", "en-US,en;q=0.9")
                .header("Referer", "https://finance.yahoo.com/")
                .send().await {
                Ok(resp) => {
                    let text = resp.text().await?;
                    
                    // Check for rate limiting
                    if text.contains("Too Many Requests") || text.contains("rate limit") {
                        if retries < 3 {
                            retries += 1;
                            tokio::time::sleep(tokio::time::Duration::from_secs(2 * retries)).await;
                            continue;
                        } else {
                            return Err(anyhow::anyhow!("Rate limited after retries"));
                        }
                    }
                    
                    break text;
                }
                Err(e) => return Err(e.into()),
            }
        };
        
        let price = self.price_regex
            .captures(&response)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract price for {}", symbol))?;
        
        let timestamp = self.time_regex
            .captures(&response)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .and_then(|ts| Local.timestamp_opt(ts, 0).single())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract timestamp for {}", symbol))?;
        
        Ok(StockPrice {
            symbol: symbol.to_string(),
            price,
            timestamp,
        })
    }
}

impl Default for YahooFinance {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires network access
    fn test_yahoo_finance() {
        let source = YahooFinance::new().unwrap();
        let price = source.fetch_price("AAPL").unwrap();
        
        assert_eq!(price.symbol, "AAPL");
        assert!(price.price > 0.0);
        println!("AAPL: ${} at {}", price.price, price.timestamp);
    }
}
