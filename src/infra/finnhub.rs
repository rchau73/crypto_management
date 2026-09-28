//! Adapter for Finnhub's `/quote` endpoint — US equities and index ETFs
//! (e.g. SPY, QQQ) or raw index symbols, depending on plan entitlements.
//! Unlike brapi, Finnhub's free-tier `/quote` endpoint takes exactly one
//! symbol per request — no batch mode — so `fetch_quotes` issues one
//! request per symbol. A symbol that fails or comes back unsupported (price
//! 0) is logged and skipped rather than failing the whole "Update Prices"
//! click over one bad ticker.

use crate::domain::market_data::{EquityProvider, MarketDataResult};
use crate::domain::models::MarketQuote;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

const QUOTE_URL: &str = "https://finnhub.io/api/v1/quote";

#[derive(Deserialize, Debug, Clone)]
struct FinnhubQuote {
    /// Current price. Finnhub returns 0 (not an error) for a symbol the
    /// caller's plan/key isn't entitled to, which is why callers must
    /// check this rather than trusting a 200 response alone.
    c: f64,
    /// Percent change on the day.
    dp: Option<f64>,
}

impl FinnhubQuote {
    fn into_market_quote(self, symbol: &str) -> Option<MarketQuote> {
        if self.c <= 0.0 {
            return None;
        }
        Some(MarketQuote {
            symbol: symbol.to_string(),
            price: self.c,
            // Finnhub's /quote endpoint has no market-cap, FDV, volume, or
            // 7d-change concept — leave the CMC-specific extras at 0.0,
            // same as every non-crypto provider (nothing downstream reads
            // them).
            market_cap: 0.0,
            fdv: 0.0,
            volume_24h: 0.0,
            percent_change_24h: self.dp.unwrap_or(0.0),
            percent_change_7d: 0.0,
        })
    }
}

pub struct FinnhubProvider {
    client: Client,
}

impl FinnhubProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for FinnhubProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EquityProvider for FinnhubProvider {
    async fn fetch_quotes(
        &self,
        api_key: &str,
        symbols: &[String],
    ) -> MarketDataResult<Vec<MarketQuote>> {
        let mut quotes = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            let response = match self
                .client
                .get(QUOTE_URL)
                .query(&[("symbol", symbol.as_str()), ("token", api_key)])
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, symbol = %symbol, "Finnhub request failed, skipping symbol");
                    continue;
                }
            };

            match response.json::<FinnhubQuote>().await {
                Ok(q) => match q.into_market_quote(symbol) {
                    Some(quote) => quotes.push(quote),
                    None => tracing::warn!(
                        symbol = %symbol,
                        "Finnhub returned a zero price (symbol likely unsupported on this plan) — skipping"
                    ),
                },
                Err(e) => {
                    tracing::warn!(error = %e, symbol = %symbol, "Failed to parse Finnhub response, skipping symbol");
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
    fn maps_a_quote_with_a_positive_price() {
        let raw = r#"{"c": 261.74, "dp": 1.15}"#;
        let parsed: FinnhubQuote = serde_json::from_str(raw).unwrap();

        let quote = parsed.into_market_quote("AAPL").unwrap();
        assert_eq!(quote.symbol, "AAPL");
        assert_eq!(quote.price, 261.74);
        assert_eq!(quote.percent_change_24h, 1.15);
        assert_eq!(quote.market_cap, 0.0);
    }

    #[test]
    fn a_zero_price_is_treated_as_unsupported_not_a_free_asset() {
        let raw = r#"{"c": 0, "dp": 0}"#;
        let parsed: FinnhubQuote = serde_json::from_str(raw).unwrap();

        assert!(parsed.into_market_quote("UNKNOWN").is_none());
    }

    #[test]
    fn a_missing_change_percent_defaults_to_zero_instead_of_failing_to_parse() {
        let raw = r#"{"c": 100.0}"#;
        let parsed: FinnhubQuote = serde_json::from_str(raw).unwrap();

        let quote = parsed.into_market_quote("SPY").unwrap();
        assert_eq!(quote.percent_change_24h, 0.0);
    }
}
