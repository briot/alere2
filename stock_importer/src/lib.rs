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
}

impl StockSource for YahooFinance {
    async fn fetch_price(&self, symbol: &str) -> Result<StockPrice> {
        let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{}", symbol);
        
        let response = self.client.get(&url)
            .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
            .header("accept-language", "en-US,en;q=0.9,fr;q=0.8")
            .header("cache-control", "max-age=0")
            .header("dnt", "1")
            .header("sec-ch-ua", r#""Not(A:Brand";v="8", "Chromium";v="144", "Google Chrome";v="144""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "document")
            .header("sec-fetch-mode", "navigate")
            .header("sec-fetch-site", "none")
            .header("sec-fetch-user", "?1")
            .header("upgrade-insecure-requests", "1")
            .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36")
            .send().await?
            .text().await?;
        
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
