//! All settings, read from the environment once at startup. A missing
//! required value stops the server immediately with a clear message,
//! instead of failing later on the first request that needs it.

use anyhow::{Context, bail};
use std::net::SocketAddr;

pub struct AdminSeed {
    pub username: String,
    pub password: String,
    pub email: String,
}

pub struct AppConfig {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub frontend_origin: String,
    pub jwt_secret: Vec<u8>,
    /// Set `Secure` on cookies (true once served over HTTPS).
    pub cookie_secure: bool,
    /// Active BARCA target profile ("BullMarket", "BearMarket"...).
    pub current_market: String,
    pub coinmarketcap_api_key: String,
    /// Optional: only needed once a br-equities asset is tracked.
    pub brapi_api_key: String,
    /// Optional: only needed once a us-indices asset is tracked.
    pub finnhub_api_key: String,
    /// CSV used to seed an empty portfolio on first start.
    pub wallet_seed_path: String,
    /// Admin account created on first start (when there are no users).
    pub admin_seed: Option<AdminSeed>,
}

fn optional(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn required(name: &str, hint: &str) -> anyhow::Result<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => bail!("{name} must be set ({hint})"),
    }
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = optional("BIND_ADDR", "127.0.0.1:3001");
        let admin_seed = match (
            std::env::var("ADMIN_USERNAME"),
            std::env::var("ADMIN_PASSWORD"),
            std::env::var("ADMIN_EMAIL"),
        ) {
            (Ok(username), Ok(password), Ok(email)) => Some(AdminSeed {
                username,
                password,
                email,
            }),
            _ => None,
        };

        Ok(Self {
            database_url: optional("DATABASE_URL", "sqlite://data/crypto.db"),
            bind_addr: bind_addr
                .parse()
                .with_context(|| format!("BIND_ADDR is not a valid address: {bind_addr}"))?,
            frontend_origin: optional("FRONTEND_ORIGIN", "http://localhost:5173"),
            jwt_secret: required("JWT_SECRET", "e.g. `openssl rand -hex 32`")?.into_bytes(),
            cookie_secure: optional("COOKIE_SECURE", "false") == "true",
            current_market: optional("CURRENT_MARKET", "BullMarket"),
            coinmarketcap_api_key: required("API_KEY", "your CoinMarketCap API key")?,
            brapi_api_key: optional("BRAPI_API_KEY", ""),
            finnhub_api_key: optional("FINNHUB_API_KEY", ""),
            wallet_seed_path: optional("WALLET_ALLOCATIONS_PATH", "wallet_allocations.csv"),
            admin_seed,
        })
    }

    /// Fixed settings for tests (no environment involved).
    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            frontend_origin: "http://localhost:5173".to_string(),
            jwt_secret: b"test-only-jwt-secret-do-not-use-in-prod".to_vec(),
            cookie_secure: false,
            current_market: "BullMarket".to_string(),
            coinmarketcap_api_key: String::new(),
            brapi_api_key: String::new(),
            finnhub_api_key: String::new(),
            wallet_seed_path: String::new(),
            admin_seed: None,
        }
    }
}
