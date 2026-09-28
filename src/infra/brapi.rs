//! Adapter for brapi.dev's `/api/quote/{tickers}` endpoint — Brazilian B3
//! stocks and FIIs (e.g. HGRU11, XPML11). Unlike CoinMarketCap's "listings"
//! endpoint, brapi has no "give me everything" mode: the caller must name
//! every ticker it wants priced, comma-separated, in one request.

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

#[async_trait]
impl EquityProvider for BrapiProvider {
    async fn fetch_quotes(
        &self,
        api_key: &str,
        symbols: &[String],
    ) -> MarketDataResult<Vec<MarketQuote>> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }
        let tickers = symbols.join(",");
        let url = format!("{QUOTE_URL}/{tickers}");

        let response = self
            .client
            .get(&url)
            .query(&[("token", api_key)])
            .send()
            .await?;

        let parsed: BrapiResponse = response.json().await?;
        Ok(parsed.results.into_iter().map(MarketQuote::from).collect())
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
