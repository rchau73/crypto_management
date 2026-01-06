use async_trait::async_trait;
use reqwest::{Client, Error as ReqwestError};
use std::collections::HashMap;

#[async_trait]
pub trait CryptoProvider: Send + Sync {
    async fn fetch_latest(&self, api_key: &str) -> Result<Vec<crate::CryptoData>, ReqwestError>;
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

#[async_trait]
impl CryptoProvider for ReqwestCryptoProvider {
    async fn fetch_latest(&self, api_key: &str) -> Result<Vec<crate::CryptoData>, ReqwestError> {
        let url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";
        let mut params = HashMap::new();
        params.insert("limit", "1000");

        let response = self
            .client
            .get(url)
            .header("X-CMC_PRO_API_KEY", api_key)
            .header("Accept", "application/json")
            .query(&params)
            .send()
            .await?;

        let parsed: crate::ApiResponse = response.json().await?;
        Ok(parsed.data)
    }
}

// Simple mock provider for tests and handler mocks
pub struct MockCryptoProvider {
    pub data: Vec<crate::CryptoData>,
}

impl MockCryptoProvider {
    #[allow(dead_code)]
    pub fn new(data: Vec<crate::CryptoData>) -> Self {
        Self { data }
    }
}

#[async_trait]
impl CryptoProvider for MockCryptoProvider {
    async fn fetch_latest(&self, _api_key: &str) -> Result<Vec<crate::CryptoData>, ReqwestError> {
        Ok(self.data.clone())
    }
}
