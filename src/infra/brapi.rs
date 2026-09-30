//! Adapter for brapi.dev's `/api/quote/{ticker}` endpoint — Brazilian B3
//! stocks and FIIs (e.g. HGRU11, XPML11). The free plan allows one ticker
//! per request, so this asks for one symbol at a time.

use crate::domain::market_data::{EquityProvider, MarketDataResult};
use crate::domain::models::MarketQuote;
use crate::infra::http::{build_client, read_json};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

const QUOTE_URL: &str = "https://brapi.dev/api/quote";

#[derive(Deserialize)]
struct BrapiResponse {
    #[serde(default)]
    results: Vec<BrapiQuote>,
}

#[derive(Deserialize)]
struct BrapiQuote {
    symbol: String,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
}

/// The first positive price in the response, if any. A missing or zero
/// price means brapi couldn't price the symbol — skip it rather than
/// valuing the holding at zero.
fn first_quote(response: BrapiResponse) -> Option<MarketQuote> {
    let quote = response.results.into_iter().next()?;
    match quote.regular_market_price {
        Some(price) if price > 0.0 => Some(MarketQuote {
            symbol: quote.symbol,
            price,
        }),
        _ => None,
    }
}

pub struct BrapiProvider {
    client: Client,
    api_key: String,
}

impl BrapiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: build_client(),
            api_key,
        }
    }

    async fn fetch_one(&self, symbol: &str) -> MarketDataResult<Option<MarketQuote>> {
        let response = self
            .client
            .get(format!("{QUOTE_URL}/{symbol}"))
            .query(&[("token", self.api_key.as_str())])
            .send()
            .await?;
        let parsed: BrapiResponse = read_json(response, "brapi").await?;
        Ok(first_quote(parsed))
    }
}

#[async_trait]
impl EquityProvider for BrapiProvider {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        // Sequential on purpose: the free plan is rate-limited, and a
        // portfolio holds only a handful of these.
        let mut quotes = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            match self.fetch_one(symbol).await {
                Ok(Some(quote)) => quotes.push(quote),
                Ok(None) => tracing::warn!(%symbol, "brapi returned no price — skipping symbol"),
                Err(e) => {
                    tracing::warn!(error = %e, %symbol, "brapi request failed — skipping symbol")
                }
            }
        }
        Ok(quotes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Option<MarketQuote> {
        first_quote(serde_json::from_str(raw).unwrap())
    }

    #[test]
    fn maps_a_brapi_quote() {
        let quote = parse(
            r#"{"results": [{"symbol": "HGRU11", "regularMarketPrice": 128.5, "marketCap": 1.0}]}"#,
        );
        assert_eq!(
            quote,
            Some(MarketQuote {
                symbol: "HGRU11".into(),
                price: 128.5
            })
        );
    }

    #[test]
    fn a_missing_or_zero_price_is_skipped_not_valued_at_zero() {
        assert_eq!(parse(r#"{"results": [{"symbol": "XPML11"}]}"#), None);
        assert_eq!(
            parse(r#"{"results": [{"symbol": "XPML11", "regularMarketPrice": 0}]}"#),
            None
        );
    }

    #[test]
    fn an_empty_or_missing_results_array_means_no_quote() {
        assert_eq!(parse(r#"{"results": []}"#), None);
        assert_eq!(parse(r#"{"error": true}"#), None);
    }
}
