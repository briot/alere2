use anyhow::Result;
use chrono::{DateTime, Local};

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
    provider: yahoo_finance_api::YahooConnector,
}

impl YahooFinance {
    pub fn new() -> Result<Self> {
        Ok(Self {
            provider: yahoo_finance_api::YahooConnector::new()?,
        })
    }
}

impl StockSource for YahooFinance {
    async fn fetch_price(&self, symbol: &str) -> Result<StockPrice> {
        let response = self.provider.get_latest_quotes(symbol, "1d").await?;
        let quote = response.last_quote()?;
        
        Ok(StockPrice {
            symbol: symbol.to_string(),
            price: quote.close,
            timestamp: DateTime::from_timestamp(quote.timestamp as i64, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?
                .with_timezone(&Local),
        })
    }
}

impl Default for YahooFinance {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
