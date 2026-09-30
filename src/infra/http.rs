//! Shared HTTP plumbing for the market-data adapters.

use crate::domain::market_data::MarketDataResult;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use std::time::Duration;

/// Without a timeout, one hung provider would hang the whole
/// "Update Prices" request forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// How much of an error body to keep in a log/error message.
const MAX_BODY_IN_ERROR: usize = 300;

pub fn build_client() -> Client {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        // Only fails if the TLS backend can't initialise; a default client
        // (no timeouts) is still better than refusing to start.
        .unwrap_or_else(|_| Client::new())
}

/// Checks the HTTP status *before* parsing. Providers return JSON error
/// bodies (bad key, rate limit...) that can parse "successfully" into an
/// empty result, which would hide the real failure.
pub async fn read_json<T: DeserializeOwned>(
    response: Response,
    provider: &str,
) -> MarketDataResult<T> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(format!(
            "{provider} request failed with {status}: {}",
            truncate(&body)
        )
        .into());
    }
    serde_json::from_str(&body).map_err(|e| {
        format!(
            "failed to parse {provider} response: {e} (body: {})",
            truncate(&body)
        )
        .into()
    })
}

fn truncate(body: &str) -> &str {
    match body.char_indices().nth(MAX_BODY_IN_ERROR) {
        Some((index, _)) => &body[..index],
        None => body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_bodies_and_cuts_long_ones_on_a_char_boundary() {
        assert_eq!(truncate("short"), "short");
        let long = "é".repeat(MAX_BODY_IN_ERROR + 10);
        assert_eq!(truncate(&long).chars().count(), MAX_BODY_IN_ERROR);
    }
}
