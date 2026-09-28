use crate::domain::models::MarketQuote;
use async_trait::async_trait;
use std::error::Error;

pub type MarketDataResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Port for fetching current market data for every crypto asset tracked
/// (CoinMarketCap's "listings/latest" returns the whole top-N market, so no
/// symbol list is needed). `infra::coinmarketcap::ReqwestCryptoProvider` is
/// the production adapter; tests use a fake implementation instead of
/// hitting the network.
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    async fn fetch_latest(&self, api_key: &str) -> MarketDataResult<Vec<MarketQuote>>;
}

/// Port for fetching current quotes for an explicit list of symbols — unlike
/// `CryptoProvider`, brapi/Finnhub have no "top N" listing endpoint, so the
/// caller must say exactly which symbols it wants priced. Shared by both the
/// Brazilian-equities (brapi) and US-indices (Finnhub) adapters since the
/// shape of "give me quotes for these symbols" is identical either way.
#[async_trait]
pub trait EquityProvider: Send + Sync {
    async fn fetch_quotes(
        &self,
        api_key: &str,
        symbols: &[String],
    ) -> MarketDataResult<Vec<MarketQuote>>;
}

/// Fake `EquityProvider` for tests — returns fixed data instead of calling
/// out to brapi or Finnhub. Shared by both adapters' consumers since the
/// trait shape is identical either way.
#[cfg(test)]
pub struct MockEquityProvider {
    pub data: Vec<MarketQuote>,
}

#[cfg(test)]
impl MockEquityProvider {
    pub fn new(data: Vec<MarketQuote>) -> Self {
        Self { data }
    }
}

#[cfg(test)]
#[async_trait]
impl EquityProvider for MockEquityProvider {
    async fn fetch_quotes(
        &self,
        _api_key: &str,
        _symbols: &[String],
    ) -> MarketDataResult<Vec<MarketQuote>> {
        Ok(self.data.clone())
    }
}
