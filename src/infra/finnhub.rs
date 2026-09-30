//! Adapter for Finnhub's `/quote` endpoint — US equities and index ETFs
//! (e.g. SPY, QQQ). The free tier takes one symbol per request.

use crate::domain::market_data::{EquityProvider, MarketDataResult};
use crate::domain::models::MarketQuote;
use crate::infra::http::{build_client, read_json};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

const QUOTE_URL: &str = "https://finnhub.io/api/v1/quote";

#[derive(Deserialize)]
struct FinnhubQuote {
    /// Current price. Finnhub answers 0 (not an error) for a symbol the
    /// key's plan can't access.
    c: f64,
}

fn to_market_quote(quote: FinnhubQuote, symbol: &str) -> Option<MarketQuote> {
    if quote.c <= 0.0 {
        return None;
    }
    Some(MarketQuote {
        symbol: symbol.to_string(),
        price: quote.c,
    })
}

pub struct FinnhubProvider {
    client: Client,
    api_key: String,
}

impl FinnhubProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: build_client(),
            api_key,
        }
    }

    async fn fetch_one(&self, symbol: &str) -> MarketDataResult<Option<MarketQuote>> {
        let response = self
            .client
            .get(QUOTE_URL)
            .query(&[("symbol", symbol), ("token", self.api_key.as_str())])
            .send()
            .await?;
        let parsed: FinnhubQuote = read_json(response, "Finnhub").await?;
        Ok(to_market_quote(parsed, symbol))
    }
}

#[async_trait]
impl EquityProvider for FinnhubProvider {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        // Sequential on purpose: the free tier is rate-limited.
        let mut quotes = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            match self.fetch_one(symbol).await {
                Ok(Some(quote)) => quotes.push(quote),
                Ok(None) => tracing::warn!(
                    %symbol,
                    "Finnhub returned a zero price (symbol likely not on this plan) — skipping"
                ),
                Err(e) => {
                    tracing::warn!(error = %e, %symbol, "Finnhub request failed — skipping symbol")
                }
            }
        }
        Ok(quotes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> FinnhubQuote {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn maps_a_positive_price() {
        let quote = to_market_quote(parse(r#"{"c": 261.74, "dp": 1.15}"#), "AAPL").unwrap();
        assert_eq!(quote.symbol, "AAPL");
        assert_eq!(quote.price, 261.74);
    }

    #[test]
    fn a_zero_price_means_unsupported_not_free() {
        assert!(to_market_quote(parse(r#"{"c": 0}"#), "UNKNOWN").is_none());
    }
}
