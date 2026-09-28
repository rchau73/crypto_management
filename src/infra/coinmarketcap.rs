//! Adapter for the CoinMarketCap "listings/latest" endpoint. Owns the
//! external API's wire format and is the only place that format is allowed
//! to leak into — everything past `fetch_latest` deals in `domain::models::Crypto`.

use crate::domain::market_data::{CryptoProvider, MarketDataResult};
use crate::domain::models::MarketQuote;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const LISTINGS_URL: &str = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ApiResponse {
    status: ApiStatus,
    data: Vec<CryptoData>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ApiStatus {
    timestamp: String,
    error_code: i32,
    error_message: Option<String>,
    credit_count: i32,
    notice: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct CryptoData {
    id: u32,
    name: String,
    symbol: String,
    cmc_rank: u32,
    tvl_ratio: Option<f64>,
    tvl_usd: Option<f64>,
    quote: QuoteData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct QuoteData {
    #[serde(rename = "USD")]
    usd: PriceInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct PriceInfo {
    price: f64,
    volume_24h: f64,
    percent_change_24h: f64,
    percent_change_7d: f64,
    market_cap: f64,
    #[serde(rename = "fully_diluted_market_cap")]
    fdv: f64,
    tvl: Option<f64>,
}

impl From<CryptoData> for MarketQuote {
    fn from(c: CryptoData) -> Self {
        MarketQuote {
            symbol: c.symbol,
            price: c.quote.usd.price,
            market_cap: c.quote.usd.market_cap,
            fdv: c.quote.usd.fdv,
            volume_24h: c.quote.usd.volume_24h,
            percent_change_24h: c.quote.usd.percent_change_24h,
            percent_change_7d: c.quote.usd.percent_change_7d,
        }
    }
}

pub struct ReqwestCryptoProvider {
    client: Client,
}

impl ReqwestCryptoProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for ReqwestCryptoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CryptoProvider for ReqwestCryptoProvider {
    async fn fetch_latest(&self, api_key: &str) -> MarketDataResult<Vec<MarketQuote>> {
        let mut params = HashMap::new();
        params.insert("limit", "1000");

        let response = self
            .client
            .get(LISTINGS_URL)
            .header("X-CMC_PRO_API_KEY", api_key)
            .header("Accept", "application/json")
            .query(&params)
            .send()
            .await?;

        let parsed: ApiResponse = response.json().await?;
        Ok(parsed.data.into_iter().map(MarketQuote::from).collect())
    }
}

/// Fake provider for tests — returns fixed data instead of calling out to CoinMarketCap.
#[cfg(test)]
pub struct MockCryptoProvider {
    pub data: Vec<MarketQuote>,
}

#[cfg(test)]
impl MockCryptoProvider {
    pub fn new(data: Vec<MarketQuote>) -> Self {
        Self { data }
    }
}

#[cfg(test)]
#[async_trait]
impl CryptoProvider for MockCryptoProvider {
    async fn fetch_latest(&self, _api_key: &str) -> MarketDataResult<Vec<MarketQuote>> {
        Ok(self.data.clone())
    }
}
