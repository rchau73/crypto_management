//! Market-data "ports". Each adapter in `infra` is built with its own API
//! key, so callers only ask for prices and never handle credentials.

use crate::domain::models::MarketQuote;
use async_trait::async_trait;

pub type MarketDataResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Prices for the whole crypto market (CoinMarketCap's "listings/latest"
/// returns the top N coins, so no symbol list is needed).
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    async fn fetch_latest(&self) -> MarketDataResult<Vec<MarketQuote>>;
}

/// Prices for an explicit list of symbols (brapi for Brazilian equities,
/// Finnhub for US indices). A symbol the provider can't price is left out
/// of the result rather than failing the whole call.
#[async_trait]
pub trait EquityProvider: Send + Sync {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>>;
}

/// Test double for both provider traits: always returns the same quotes,
/// or always fails when built with `failing()`.
#[cfg(test)]
pub struct FakeProvider {
    quotes: Vec<MarketQuote>,
    fail: bool,
}

#[cfg(test)]
impl FakeProvider {
    pub fn with_prices(prices: &[(&str, f64)]) -> Self {
        let quotes = prices
            .iter()
            .map(|(symbol, price)| MarketQuote {
                symbol: symbol.to_string(),
                price: *price,
            })
            .collect();
        Self {
            quotes,
            fail: false,
        }
    }

    pub fn empty() -> Self {
        Self::with_prices(&[])
    }

    pub fn failing() -> Self {
        Self {
            quotes: vec![],
            fail: true,
        }
    }

    fn result(&self) -> MarketDataResult<Vec<MarketQuote>> {
        if self.fail {
            return Err("provider is down".into());
        }
        Ok(self.quotes.clone())
    }
}

#[cfg(test)]
#[async_trait]
impl CryptoProvider for FakeProvider {
    async fn fetch_latest(&self) -> MarketDataResult<Vec<MarketQuote>> {
        self.result()
    }
}

#[cfg(test)]
#[async_trait]
impl EquityProvider for FakeProvider {
    async fn fetch_quotes(&self, _symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        self.result()
    }
}
