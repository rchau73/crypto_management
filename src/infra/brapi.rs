//! Adapter for brapi.dev's `/api/quote/{ticker}` endpoint — Brazilian B3
//! stocks and FIIs (e.g. HGRU11, XPML11). brapi supports batching several
//! comma-separated tickers into one request on paid plans, but the free
//! plan caps that at 1 ticker per request (a 2nd symbol gets a 400
//! QUOTES_PER_REQUEST_EXCEEDED) — so, like Finnhub, this issues one request
//! per symbol rather than assuming a batch-capable plan.

use crate::domain::market_data::{EquityProvider, MarketDataResult};
use crate::domain::models::MarketQuote;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

const QUOTE_URL: &str = "https://brapi.dev/api/quote";

#[derive(Deserialize, Debug, Clone)]
struct BrapiResponse {
    #[serde(default)]
    results: Vec<BrapiQuote>,
}

#[derive(Deserialize, Debug, Clone)]
struct BrapiQuote {
    symbol: String,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketChangePercent")]
    regular_market_change_percent: Option<f64>,
    #[serde(rename = "regularMarketVolume")]
    regular_market_volume: Option<f64>,
    #[serde(rename = "marketCap")]
    market_cap: Option<f64>,
}

impl From<BrapiQuote> for MarketQuote {
    fn from(q: BrapiQuote) -> Self {
        MarketQuote {
            symbol: q.symbol,
            price: q.regular_market_price.unwrap_or(0.0),
            market_cap: q.market_cap.unwrap_or(0.0),
            // brapi's quote endpoint has no fully-diluted-valuation or 7d
            // change concept — leave the CMC-specific extras at 0.0, same
            // as every non-crypto provider (nothing downstream reads them).
            fdv: 0.0,
            volume_24h: q.regular_market_volume.unwrap_or(0.0),
            percent_change_24h: q.regular_market_change_percent.unwrap_or(0.0),
            percent_change_7d: 0.0,
        }
    }
}

pub struct BrapiProvider {
    client: Client,
}

impl BrapiProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for BrapiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BrapiProvider {
    /// One symbol, one request — see the module doc comment for why. Returns
    /// `Ok(None)` (not an error) for a symbol brapi didn't return a quote
    /// for, mirroring `FinnhubQuote::into_market_quote`'s "zero price isn't
    /// an error" handling.
    async fn fetch_one(
        &self,
        api_key: &str,
        symbol: &str,
    ) -> MarketDataResult<Option<MarketQuote>> {
        let url = format!("{QUOTE_URL}/{symbol}");
        let response = self
            .client
            .get(&url)
            .query(&[("token", api_key)])
            .send()
            .await?;

        // brapi returns a JSON body on errors too (bad token, plan limits,
        // ...), just shaped differently (no "results" array). Since
        // `results` defaults to empty, parsing that body as `BrapiResponse`
        // without checking the status first would silently succeed with zero
        // quotes — the caller logs nothing, and the asset just looks
        // untracked, with no clue why. Surface the status/body as a real
        // error instead so it actually reaches the caller's log line.
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(format!("brapi request failed with {status}: {body}").into());
        }

        let parsed: BrapiResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse brapi response: {e} (body: {body})"))?;
        Ok(parsed.results.into_iter().next().map(MarketQuote::from))
    }
}

#[async_trait]
impl EquityProvider for BrapiProvider {
    async fn fetch_quotes(
        &self,
        api_key: &str,
        symbols: &[String],
    ) -> MarketDataResult<Vec<MarketQuote>> {
        let mut quotes = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            match self.fetch_one(api_key, symbol).await {
                Ok(Some(quote)) => quotes.push(quote),
                Ok(None) => {
                    tracing::warn!(symbol = %symbol, "brapi returned no quote for symbol — skipping")
                }
                Err(e) => {
                    tracing::warn!(error = %e, symbol = %symbol, "brapi request failed, skipping symbol")
                }
            }
        }
        Ok(quotes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_a_full_brapi_quote_into_a_market_quote() {
        let raw = r#"{
            "results": [
                {
                    "symbol": "HGRU11",
                    "regularMarketPrice": 128.5,
                    "regularMarketChangePercent": -0.42,
                    "regularMarketVolume": 15234.0,
                    "marketCap": 987654321.0
                }
            ]
        }"#;

        let parsed: BrapiResponse = serde_json::from_str(raw).unwrap();
        let quote: MarketQuote = parsed.results.into_iter().next().unwrap().into();

        assert_eq!(quote.symbol, "HGRU11");
        assert_eq!(quote.price, 128.5);
        assert_eq!(quote.market_cap, 987654321.0);
        assert_eq!(quote.volume_24h, 15234.0);
        assert_eq!(quote.percent_change_24h, -0.42);
        assert_eq!(quote.fdv, 0.0);
        assert_eq!(quote.percent_change_7d, 0.0);
    }

    #[test]
    fn missing_optional_fields_default_to_zero_instead_of_failing_to_parse() {
        let raw = r#"{"results": [{"symbol": "XPML11"}]}"#;

        let parsed: BrapiResponse = serde_json::from_str(raw).unwrap();
        let quote: MarketQuote = parsed.results.into_iter().next().unwrap().into();

        assert_eq!(quote.symbol, "XPML11");
        assert_eq!(quote.price, 0.0);
        assert_eq!(quote.market_cap, 0.0);
    }

    #[test]
    fn an_empty_results_array_maps_to_no_quotes() {
        let raw = r#"{"results": []}"#;
        let parsed: BrapiResponse = serde_json::from_str(raw).unwrap();
        assert!(parsed.results.is_empty());
    }
}
