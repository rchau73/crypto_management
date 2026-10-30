//! Adapter for CoinMarketCap's "listings/latest" endpoint. Only the fields
//! we use are declared, so an unrelated field changing shape (or being
//! null for some coin) can't break the whole price refresh.
//!
//! Tickers are not unique on CoinMarketCap: in October 2026 a memecoin
//! also called "BTC" entered the top 1000 and Bitcoin got priced at $0.03.
//! The listing is sorted by market cap, so for a repeated ticker we keep
//! the first (largest) coin and drop the rest.

use crate::domain::market_data::{CryptoProvider, MarketDataResult};
use crate::domain::models::MarketQuote;
use crate::infra::http::{build_client, read_json};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;

const LISTINGS_URL: &str = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";
const LISTING_LIMIT: &str = "1000";

#[derive(Deserialize)]
struct ListingsResponse {
    data: Vec<Listing>,
}

#[derive(Deserialize)]
struct Listing {
    symbol: String,
    #[serde(default)]
    name: Option<String>,
    quote: Quote,
}

#[derive(Deserialize)]
struct Quote {
    #[serde(rename = "USD")]
    usd: UsdQuote,
}

#[derive(Deserialize)]
struct UsdQuote {
    price: Option<f64>,
}

fn to_market_quotes(response: ListingsResponse) -> Vec<MarketQuote> {
    let mut seen = HashSet::new();
    let mut quotes = Vec::new();
    for listing in response.data {
        // A listed coin with no price can't be valued — leave it out.
        let Some(price) = listing.quote.usd.price else {
            continue;
        };
        if !seen.insert(listing.symbol.clone()) {
            tracing::warn!(
                symbol = %listing.symbol,
                name = listing.name.as_deref().unwrap_or(""),
                price,
                "CoinMarketCap lists this ticker twice — keeping the larger coin"
            );
            continue;
        }
        quotes.push(MarketQuote {
            symbol: listing.symbol,
            price,
        });
    }
    quotes
}

pub struct CoinMarketCapProvider {
    client: Client,
    api_key: String,
}

impl CoinMarketCapProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: build_client(),
            api_key,
        }
    }
}

#[async_trait]
impl CryptoProvider for CoinMarketCapProvider {
    async fn fetch_latest(&self) -> MarketDataResult<Vec<MarketQuote>> {
        let response = self
            .client
            .get(LISTINGS_URL)
            .header("X-CMC_PRO_API_KEY", &self.api_key)
            .header("Accept", "application/json")
            .query(&[("limit", LISTING_LIMIT)])
            .send()
            .await?;
        let parsed: ListingsResponse = read_json(response, "CoinMarketCap").await?;
        Ok(to_market_quotes(parsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_listings_and_ignores_fields_we_do_not_use() {
        let raw = r#"{
            "status": {"error_code": 0},
            "data": [
                {"symbol": "BTC", "name": "Bitcoin", "quote": {"USD": {"price": 65000.5, "fully_diluted_market_cap": null}}},
                {"symbol": "ETH", "quote": {"USD": {"price": 3000.0}}}
            ]
        }"#;
        let quotes = to_market_quotes(serde_json::from_str(raw).unwrap());
        assert_eq!(
            quotes,
            vec![
                MarketQuote {
                    symbol: "BTC".into(),
                    price: 65000.5
                },
                MarketQuote {
                    symbol: "ETH".into(),
                    price: 3000.0
                },
            ]
        );
    }

    #[test]
    fn a_repeated_ticker_keeps_the_first_larger_coin() {
        // Regression: "Big Titty Coin" (rank 569, $0.03) also uses BTC and
        // overwrote Bitcoin's price.
        let raw = r#"{"data": [
            {"symbol": "BTC", "name": "Bitcoin", "quote": {"USD": {"price": 79000.0}}},
            {"symbol": "ETH", "quote": {"USD": {"price": 3000.0}}},
            {"symbol": "BTC", "name": "Big Titty Coin", "quote": {"USD": {"price": 0.03}}}
        ]}"#;
        let quotes = to_market_quotes(serde_json::from_str(raw).unwrap());
        assert_eq!(
            quotes,
            vec![
                MarketQuote {
                    symbol: "BTC".into(),
                    price: 79000.0
                },
                MarketQuote {
                    symbol: "ETH".into(),
                    price: 3000.0
                },
            ]
        );
    }

    #[test]
    fn a_priceless_first_listing_does_not_hide_a_priced_one() {
        let raw = r#"{"data": [
            {"symbol": "X", "quote": {"USD": {"price": null}}},
            {"symbol": "X", "quote": {"USD": {"price": 2.0}}}
        ]}"#;
        let quotes = to_market_quotes(serde_json::from_str(raw).unwrap());
        assert_eq!(
            quotes,
            vec![MarketQuote {
                symbol: "X".into(),
                price: 2.0
            }]
        );
    }

    #[test]
    fn a_listing_with_a_null_price_is_skipped_not_priced_at_zero() {
        let raw = r#"{"data": [{"symbol": "NEW", "quote": {"USD": {"price": null}}}]}"#;
        assert!(to_market_quotes(serde_json::from_str(raw).unwrap()).is_empty());
    }
}
