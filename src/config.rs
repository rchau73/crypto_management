//! All settings, read from the environment once at startup. A missing
//! required value stops the server immediately with a clear message,
//! instead of failing later on the first request that needs it.

use anyhow::{Context, bail};
use std::net::SocketAddr;
use std::time::Duration;

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
    /// Prepended to the auth cookie names (`cm` -> `cm_access_token`).
    /// Browsers share cookies across ports, so two apps on `localhost`
    /// with the same cookie names log each other out.
    pub cookie_prefix: String,
    /// Active BARCA target profile ("BullMarket", "BearMarket"...).
    pub current_market: String,
    /// How old the shared prices may get before the next request refreshes
    /// them (`PRICE_REFRESH_MINUTES`, default 5). CoinMarketCap's free plan
    /// has 10,000 credits a month and a refresh costs 1 per 100 symbols, so
    /// 5 minutes of constant use stays within it.
    pub price_refresh: Duration,
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

/// Cookie names may only use a safe subset of characters; reject anything
/// else at startup rather than emitting a cookie the browser ignores.
fn cookie_prefix(raw: &str) -> anyhow::Result<String> {
    let valid = raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid {
        bail!("COOKIE_PREFIX may only contain letters, digits, '_' or '-': {raw:?}");
    }
    Ok(raw.to_string())
}

fn price_refresh(raw: &str) -> anyhow::Result<Duration> {
    match raw.trim().parse::<u64>() {
        Ok(minutes) if minutes >= 1 => Ok(Duration::from_secs(minutes * 60)),
        _ => bail!("PRICE_REFRESH_MINUTES must be a whole number of minutes, at least 1: {raw:?}"),
    }
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
            cookie_prefix: cookie_prefix(&optional("COOKIE_PREFIX", "cm"))?,
            current_market: optional("CURRENT_MARKET", "BullMarket"),
            price_refresh: price_refresh(&optional("PRICE_REFRESH_MINUTES", "5"))?,
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
            cookie_prefix: "cm".to_string(),
            current_market: "BullMarket".to_string(),
            // Every request refreshes, like the old "Update Prices"; the
            // caching itself is tested in allocations_service.
            price_refresh: Duration::ZERO,
            coinmarketcap_api_key: String::new(),
            brapi_api_key: String::new(),
            finnhub_api_key: String::new(),
            wallet_seed_path: String::new(),
            admin_seed: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_prefix_accepts_safe_names_and_empty() {
        assert_eq!(cookie_prefix("crypto-mgmt_2").unwrap(), "crypto-mgmt_2");
        assert_eq!(cookie_prefix("").unwrap(), "");
    }

    #[test]
    fn price_refresh_is_whole_minutes_of_at_least_one() {
        assert_eq!(price_refresh("5").unwrap(), Duration::from_secs(300));
        assert_eq!(price_refresh(" 30 ").unwrap(), Duration::from_secs(1800));
        for bad in ["0", "-1", "0.5", "ten", ""] {
            assert!(price_refresh(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn cookie_prefix_rejects_characters_invalid_in_a_cookie_name() {
        for bad in ["a b", "a;b", "a=b", "café", "a,b"] {
            assert!(cookie_prefix(bad).is_err(), "{bad:?} should be rejected");
        }
    }
}
