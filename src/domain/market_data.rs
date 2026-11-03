//! Market-data "ports". Each adapter in `infra` is built with its own API
//! key, so callers only ask for prices and never handle credentials.

use crate::domain::models::{FxRate, MarketQuote};
use async_trait::async_trait;

pub type MarketDataResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Crypto prices for an explicit list of symbols (CoinMarketCap). Only the
/// held symbols are asked for, since the API bills per symbol batch. A
/// symbol it can't price is left out of the result.
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>>;
}

/// Prices for an explicit list of symbols (brapi for Brazilian equities,
/// Finnhub for US indices). A symbol the provider can't price is left out
/// of the result rather than failing the whole call.
#[async_trait]
pub trait EquityProvider: Send + Sync {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>>;
}

/// The USD/BRL exchange rate used to convert Brazilian quotes to dollars
/// (the Banco Central's PTAX selling rate).
#[async_trait]
pub trait FxProvider: Send + Sync {
    async fn fetch_usd_brl(&self) -> MarketDataResult<FxRate>;
}

/// Test double for both provider traits: always returns the same quotes
/// (only those asked for), or always fails when built with `failing()`.
/// `calls()` counts the requests, to test that prices are cached.
#[cfg(test)]
pub struct FakeProvider {
    quotes: Vec<MarketQuote>,
    fail: bool,
    calls: std::sync::atomic::AtomicUsize,
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
            calls: Default::default(),
        }
    }

    pub fn empty() -> Self {
        Self::with_prices(&[])
    }

    pub fn failing() -> Self {
        Self {
            quotes: vec![],
            fail: true,
            calls: Default::default(),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn result(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail {
            return Err("provider is down".into());
        }
        Ok(self
            .quotes
            .iter()
            .filter(|q| symbols.contains(&q.symbol))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
#[async_trait]
impl CryptoProvider for FakeProvider {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        self.result(symbols)
    }
}

#[cfg(test)]
#[async_trait]
impl EquityProvider for FakeProvider {
    async fn fetch_quotes(&self, symbols: &[String]) -> MarketDataResult<Vec<MarketQuote>> {
        self.result(symbols)
    }
}

/// Test double for `FxProvider`: a fixed rate, or always fails.
#[cfg(test)]
pub struct FakeFx(pub Option<f64>);

#[cfg(test)]
#[async_trait]
impl FxProvider for FakeFx {
    async fn fetch_usd_brl(&self) -> MarketDataResult<FxRate> {
        match self.0 {
            Some(brl_per_usd) => Ok(FxRate {
                brl_per_usd,
                quoted_at: "2026-10-01 13:10:35".to_string(),
            }),
            None => Err("PTAX is down".into()),
        }
    }
}
