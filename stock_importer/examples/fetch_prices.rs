use stock_importer::{StockSource, YahooFinance};

fn main() {
    let source = YahooFinance::new().expect("Failed to create Yahoo Finance source");
    
    let symbols = vec!["AAPL", "MSFT", "GOOGL"];
    
    for symbol in symbols {
        match source.fetch_price(symbol) {
            Ok(price) => {
                println!("{}: ${:.2} at {}", 
                    price.symbol, 
                    price.price, 
                    price.timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            Err(e) => {
                eprintln!("Error fetching {}: {}", symbol, e);
            }
        }
    }
}
