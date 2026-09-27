use crate::domain::models::Crypto;
use async_trait::async_trait;
use std::error::Error;

pub type MarketDataResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Port for fetching current market data for all tracked assets.
/// `infra::coinmarketcap::ReqwestCryptoProvider` is the production adapter;
/// tests use a fake implementation instead of hitting the network.
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    async fn fetch_latest(&self, api_key: &str) -> MarketDataResult<Vec<Crypto>>;
}
