//! Adapter for CoinMarketCap's v2 "quotes/latest" endpoint: prices for the
//! symbols we hold only, at 1 credit per 100 symbols (the old top-1000
//! "listings/latest" call cost 5 credits whatever we held). Only the fields
//! we use are declared, so an unrelated field changing shape (or being null
//! for some coin) can't break the whole price refresh.
//!
//! Tickers are not unique on CoinMarketCap: v2 answers every symbol with a
//! list of all the coins using it (14 "BTC"s in November 2026, one of which
//! once got Bitcoin priced at $0.03). We keep the active, priced coin with
//! the best market-cap rank.

use crate::domain::market_data::{CryptoProvider, MarketDataResult};
use crate::domain::models::MarketQuote;
use crate::infra::http::{build_client, read_json};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const QUOTES_URL: &str = "https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest";
/// One credit buys up to 100 symbols, so ask in batches of that size.
const SYMBOLS_PER_CALL: usize = 100;

#[derive(Deserialize)]
struct QuotesResponse {
    /// Symbol as asked -> every coin using that ticker.
    data: HashMap<String, Vec<Coin>>,
}

#[derive(Deserialize)]
struct Coin {
    #[serde(default)]
    name: Option<String>,
    /// Market-cap rank (1 = largest); null for coins too small to rank.
    #[serde(default)]
    cmc_rank: Option<u32>,
    #[serde(default)]
    is_active: Option<u8>,
    #[serde(default)]
    quote: Option<Quote>,
}

#[derive(Deserialize)]
struct Quote {
    #[serde(rename = "USD")]
    usd: Option<UsdQuote>,
}

#[derive(Deserialize)]
struct UsdQuote {
    price: Option<f64>,
}

impl Coin {
    fn price(&self) -> Option<f64> {
        self.quote.as_ref()?.usd.as_ref()?.price
    }
}

/// One quote per symbol: the active, priced coin with the best rank. A
/// symbol no coin can price is left out (it shows as unpriced in the app,
/// rather than at $0).
fn to_market_quotes(response: QuotesResponse) -> Vec<MarketQuote> {
    let mut quotes: Vec<MarketQuote> = response
        .data
        .into_iter()
        .filter_map(|(symbol, coins)| {
            let candidates = coins.len();
            let best = coins
                .into_iter()
                .filter(|c| c.is_active != Some(0))
                .filter_map(|c| c.price().map(|price| (c, price)))
                .min_by_key(|(c, _)| c.cmc_rank.unwrap_or(u32::MAX))?;
            if candidates > 1 {
                tracing::debug!(
                    %symbol,
                    candidates,
                    chosen = best.0.name.as_deref().unwrap_or(""),
                    rank = ?best.0.cmc_rank,
                    "Ticker shared by several coins — kept the best-ranked one"
                );
            }
            Some(MarketQuote {
                symbol,
                price: best.1,
            })
        })
        .collect();
    quotes.sort_by(|a, b| a.symbol.cmp(&b.symbol));
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

    async fn fetch_batch(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        let response = self
            .client
            .get(QUOTES_URL)
            .header("X-CMC_PRO_API_KEY", &self.api_key)
            .header("Accept", "application/json")
            // skip_invalid: an unknown symbol is dropped instead of failing
            // the whole call.
            .query(&[
                ("symbol", symbols.join(",").as_str()),
                ("skip_invalid", "true"),
            ])
            .send()
            .await?;
        let parsed: QuotesResponse = read_json(response, "CoinMarketCap").await?;
        Ok(to_market_quotes(parsed))
    }
}

/// CoinMarketCap rejects the whole call (400) if any symbol has anything
/// but letters and digits, so such a symbol (e.g. "#RAY") is left out and
/// stays unpriced, as it was with the old listing call.
fn askable(symbols: &[String]) -> (Vec<String>, Vec<&str>) {
    let (ok, bad): (Vec<&String>, Vec<&String>) = symbols
        .iter()
        .partition(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric()));
    (
        ok.into_iter().cloned().collect(),
        bad.into_iter().map(String::as_str).collect(),
    )
}

#[async_trait]
impl CryptoProvider for CoinMarketCapProvider {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        let (symbols, skipped) = askable(symbols);
        if !skipped.is_empty() {
            tracing::warn!(
                ?skipped,
                "Not asking CoinMarketCap for symbols that aren't letters and digits — they stay unpriced"
            );
        }
        let mut quotes = Vec::new();
        for batch in symbols.chunks(SYMBOLS_PER_CALL) {
            quotes.extend(self.fetch_batch(batch).await?);
        }
        tracing::info!(
            symbols = symbols.len(),
            priced = quotes.len(),
            calls = symbols.len().div_ceil(SYMBOLS_PER_CALL),
            "Fetched CoinMarketCap quotes"
        );
        Ok(quotes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quotes(raw: &str) -> Vec<MarketQuote> {
        to_market_quotes(serde_json::from_str(raw).unwrap())
    }

    fn quote(symbol: &str, price: f64) -> MarketQuote {
        MarketQuote {
            symbol: symbol.into(),
            price,
        }
    }

    #[test]
    fn symbols_that_would_make_coinmarketcap_reject_the_call_are_left_out() {
        // Regression: a "#RAY" position made every refresh fail with 400.
        let symbols = ["BTC", "#RAY", "ETH", "", "A B", "1INCH"].map(String::from);
        let (ok, skipped) = askable(&symbols);
        assert_eq!(ok, ["BTC", "ETH", "1INCH"]);
        assert_eq!(skipped, ["#RAY", "", "A B"]);
    }

    #[test]
    fn maps_quotes_and_ignores_fields_we_do_not_use() {
        let raw = r#"{
            "status": {"error_code": 0, "credit_count": 1},
            "data": {
                "BTC": [{"id": 1, "name": "Bitcoin", "cmc_rank": 1, "is_active": 1,
                         "quote": {"USD": {"price": 65000.5, "fully_diluted_market_cap": null}}}],
                "ETH": [{"name": "Ethereum", "cmc_rank": 2, "quote": {"USD": {"price": 3000.0}}}]
            }
        }"#;
        assert_eq!(
            quotes(raw),
            vec![quote("BTC", 65000.5), quote("ETH", 3000.0)]
        );
    }

    #[test]
    fn a_shared_ticker_keeps_the_best_ranked_coin_whatever_the_order() {
        // Regression: "Big Titty Coin" also uses BTC and once priced
        // Bitcoin at $0.03.
        let raw = r#"{"data": {"BTC": [
            {"name": "Big Titty Coin", "cmc_rank": 3840, "is_active": 1, "quote": {"USD": {"price": 0.03}}},
            {"name": "Bitcoin", "cmc_rank": 1, "is_active": 1, "quote": {"USD": {"price": 79000.0}}},
            {"name": "Unranked", "cmc_rank": null, "is_active": 1, "quote": {"USD": {"price": 5.0}}}
        ]}}"#;
        assert_eq!(quotes(raw), vec![quote("BTC", 79000.0)]);
    }

    #[test]
    fn inactive_and_priceless_coins_are_never_chosen() {
        let raw = r#"{"data": {"ETH": [
            {"name": "Dead", "cmc_rank": 1, "is_active": 0, "quote": {"USD": {"price": 1.0}}},
            {"name": "No price", "cmc_rank": 2, "is_active": 1, "quote": {"USD": {"price": null}}},
            {"name": "No quote", "cmc_rank": 3, "is_active": 1, "quote": {}},
            {"name": "Ethereum", "cmc_rank": 4, "is_active": 1, "quote": {"USD": {"price": 3000.0}}}
        ]}}"#;
        assert_eq!(quotes(raw), vec![quote("ETH", 3000.0)]);
    }

    #[test]
    fn an_unknown_or_unpriced_symbol_is_left_out_not_priced_at_zero() {
        let raw = r#"{"data": {
            "NOTACOIN": [],
            "NEW": [{"cmc_rank": null, "is_active": 0, "quote": {"USD": {"price": null}}}]
        }}"#;
        assert!(quotes(raw).is_empty());
    }
}
