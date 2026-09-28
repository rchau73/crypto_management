use crate::domain::market_data::{CryptoProvider, EquityProvider};
use crate::domain::models::AssetClass;
use crate::domain::repository::{BarcaTargetRepo, HistoryRepo};
use crate::usecases::compute_allocations::compute_allocations;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct AllocationsService {
    pub crypto_provider: Arc<dyn CryptoProvider>,
    pub br_equity_provider: Arc<dyn EquityProvider>,
    pub us_equity_provider: Arc<dyn EquityProvider>,
    pub repo: Arc<dyn HistoryRepo>,
    pub barca_target_repo: Arc<dyn BarcaTargetRepo>,
}

impl AllocationsService {
    pub fn new(
        crypto_provider: Arc<dyn CryptoProvider>,
        br_equity_provider: Arc<dyn EquityProvider>,
        us_equity_provider: Arc<dyn EquityProvider>,
        repo: Arc<dyn HistoryRepo>,
        barca_target_repo: Arc<dyn BarcaTargetRepo>,
    ) -> Self {
        Self {
            crypto_provider,
            br_equity_provider,
            us_equity_provider,
            repo,
            barca_target_repo,
        }
    }

    /// Fetches fresh prices from every configured provider and recomputes
    /// allocations — this is the entire "Update Prices" flow, triggered
    /// only by the user pressing that one button (no polling, no
    /// websockets). Crypto pricing is required (CoinMarketCap is the
    /// original, always-on data source); a br-equities or us-indices
    /// provider failure is logged and skipped rather than failing the
    /// whole update, since either is an optional diversification the user
    /// may not have API keys for yet.
    pub async fn compute_and_record(
        &self,
        crypto_api_key: &str,
        br_equity_api_key: &str,
        us_equity_api_key: &str,
        current_market: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let allocs = self.repo.fetch_current_wallet_allocations().await?;

        let mut quotes = self.crypto_provider.fetch_latest(crypto_api_key).await?;

        let br_symbols = symbols_for_asset_class(&allocs, AssetClass::BrEquities);
        if !br_symbols.is_empty() {
            match self
                .br_equity_provider
                .fetch_quotes(br_equity_api_key, &br_symbols)
                .await
            {
                Ok(mut q) => quotes.append(&mut q),
                Err(e) => tracing::error!(
                    error = %e,
                    symbols = ?br_symbols,
                    "Failed to fetch br-equities quotes — those assets will price as untracked this round"
                ),
            }
        }

        let us_symbols = symbols_for_asset_class(&allocs, AssetClass::UsIndices);
        if !us_symbols.is_empty() {
            match self
                .us_equity_provider
                .fetch_quotes(us_equity_api_key, &us_symbols)
                .await
            {
                Ok(mut q) => quotes.append(&mut q),
                Err(e) => tracing::error!(
                    error = %e,
                    symbols = ?us_symbols,
                    "Failed to fetch us-indices quotes — those assets will price as untracked this round"
                ),
            }
        }

        // BARCA targets are DB-authoritative (editable via the BARCA admin
        // UI), scoped to the currently-active market profile — see
        // migrations/0006 for why there's no cross-market fallback.
        let barca_targets: HashMap<String, f64> = self
            .barca_target_repo
            .fetch_barca_targets(current_market)
            .await?
            .into_iter()
            .map(|t| (t.barca, t.target_percent))
            .collect();

        let res = compute_allocations(&allocs, &quotes, &barca_targets);

        // persist computed allocation record for audit
        let rec = crate::domain::models::AllocationRecord {
            id: None,
            computed_at: chrono::Utc::now().to_rfc3339(),
            payload: res.clone(),
            created_at: None,
        };
        self.repo.persist_allocation_record(&rec).await?;

        Ok(res)
    }
}

fn symbols_for_asset_class(
    allocs: &[crate::domain::models::WalletAllocation],
    class: AssetClass,
) -> Vec<String> {
    allocs
        .iter()
        .filter(|a| a.asset_class == class.as_str())
        .map(|a| a.symbol.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::market_data::{MarketDataResult, MockEquityProvider};
    use crate::domain::models::{MarketQuote, WalletAllocation};
    use crate::domain::repository::BarcaTargetInput;
    use crate::infra::coinmarketcap::MockCryptoProvider;
    use crate::infra::sqlite::SqliteRepo;
    use async_trait::async_trait;
    use sqlx::SqlitePool;

    async fn in_memory_repo() -> Arc<SqliteRepo> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        Arc::new(SqliteRepo::new(pool))
    }

    fn wallet_row(symbol: &str, asset_class: &str, target: f64, qty: f64) -> WalletAllocation {
        WalletAllocation {
            id: None,
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            target_percent: Some(target),
            current_quantity: Some(qty),
            last_price: None,
            notes: None,
            asset_class: asset_class.to_string(),
            created_at: None,
        }
    }

    struct FailingEquityProvider;

    #[async_trait]
    impl EquityProvider for FailingEquityProvider {
        async fn fetch_quotes(
            &self,
            _api_key: &str,
            _symbols: &[String],
        ) -> MarketDataResult<Vec<MarketQuote>> {
            Err("brapi is down".into())
        }
    }

    fn quote(symbol: &str, price: f64) -> MarketQuote {
        MarketQuote {
            symbol: symbol.to_string(),
            price,
            market_cap: 0.0,
            fdv: 0.0,
            volume_24h: 0.0,
            percent_change_24h: 0.0,
            percent_change_7d: 0.0,
        }
    }

    #[tokio::test]
    async fn merges_crypto_and_br_equity_quotes_before_computing_allocations() {
        let repo = in_memory_repo().await;
        repo.insert_wallet_allocation(&wallet_row("BTC", "crypto", 50.0, 1.0))
            .await
            .unwrap();
        repo.insert_wallet_allocation(&wallet_row("HGRU11", "br-equities", 50.0, 10.0))
            .await
            .unwrap();
        repo.replace_barca_targets(
            "BullMarket",
            &[BarcaTargetInput {
                barca: "Base",
                target_percent: 100.0,
            }],
            None,
        )
        .await
        .unwrap();

        let service = AllocationsService::new(
            Arc::new(MockCryptoProvider::new(vec![quote("BTC", 10.0)])),
            Arc::new(MockEquityProvider::new(vec![quote("HGRU11", 100.0)])),
            Arc::new(MockEquityProvider::new(vec![])),
            repo.clone(),
            repo.clone(),
        );

        let result = service
            .compute_and_record("crypto-key", "brapi-key", "finnhub-key", "BullMarket")
            .await
            .unwrap();

        let per_asset = result["per_asset"].as_array().unwrap();
        assert_eq!(per_asset.len(), 2);
        let btc_value = per_asset.iter().find(|a| a["symbol"] == "BTC").unwrap()["value"]
            .as_f64()
            .unwrap();
        let hgru_value = per_asset.iter().find(|a| a["symbol"] == "HGRU11").unwrap()["value"]
            .as_f64()
            .unwrap();
        assert_eq!(btc_value, 10.0); // 1.0 qty * 10.0 price
        assert_eq!(hgru_value, 1000.0); // 10.0 qty * 100.0 price
    }

    #[tokio::test]
    async fn a_br_equity_provider_failure_does_not_abort_the_whole_update() {
        let repo = in_memory_repo().await;
        repo.insert_wallet_allocation(&wallet_row("BTC", "crypto", 100.0, 1.0))
            .await
            .unwrap();
        repo.insert_wallet_allocation(&wallet_row("HGRU11", "br-equities", 0.0, 10.0))
            .await
            .unwrap();
        repo.replace_barca_targets(
            "BullMarket",
            &[BarcaTargetInput {
                barca: "Base",
                target_percent: 100.0,
            }],
            None,
        )
        .await
        .unwrap();

        let service = AllocationsService::new(
            Arc::new(MockCryptoProvider::new(vec![quote("BTC", 10.0)])),
            Arc::new(FailingEquityProvider),
            Arc::new(MockEquityProvider::new(vec![])),
            repo.clone(),
            repo.clone(),
        );

        // The br-equities provider errors out entirely, but the crypto side
        // must still succeed rather than the whole "Update Prices" click failing.
        let result = service
            .compute_and_record("crypto-key", "brapi-key", "finnhub-key", "BullMarket")
            .await
            .unwrap();

        let per_asset = result["per_asset"].as_array().unwrap();
        assert!(per_asset.iter().any(|a| a["symbol"] == "BTC"));
        // HGRU11 has no wallet_allocations price fed to compute_allocations,
        // so it's silently dropped (unknown_symbol_is_dropped_silently — see
        // compute_allocations tests), same as any other unpriced asset.
        assert!(!per_asset.iter().any(|a| a["symbol"] == "HGRU11"));
    }

    #[tokio::test]
    async fn barca_targets_come_from_the_db_for_the_requested_market_only() {
        let repo = in_memory_repo().await;
        repo.insert_wallet_allocation(&wallet_row("BTC", "crypto", 100.0, 1.0))
            .await
            .unwrap();
        repo.replace_barca_targets(
            "BullMarket",
            &[BarcaTargetInput {
                barca: "Base",
                target_percent: 100.0,
            }],
            None,
        )
        .await
        .unwrap();
        repo.replace_barca_targets(
            "BearMarket",
            &[BarcaTargetInput {
                barca: "Base",
                target_percent: 40.0,
            }],
            None,
        )
        .await
        .unwrap();

        let service = AllocationsService::new(
            Arc::new(MockCryptoProvider::new(vec![quote("BTC", 10.0)])),
            Arc::new(MockEquityProvider::new(vec![])),
            Arc::new(MockEquityProvider::new(vec![])),
            repo.clone(),
            repo.clone(),
        );

        let result = service
            .compute_and_record("crypto-key", "", "", "BearMarket")
            .await
            .unwrap();

        let per_barca = result["per_barca"].as_array().unwrap();
        let base = per_barca.iter().find(|b| b["barca"] == "Base").unwrap();
        assert_eq!(base["target_percent"].as_f64().unwrap(), 40.0);
    }

    #[test]
    fn symbols_for_asset_class_filters_and_dedupes() {
        let allocs = vec![
            wallet_row("HGRU11", "br-equities", 10.0, 1.0),
            wallet_row("HGRU11", "br-equities", 5.0, 2.0), // duplicate symbol/asset_class
            wallet_row("BTC", "crypto", 50.0, 1.0),
        ];

        let mut symbols = symbols_for_asset_class(&allocs, AssetClass::BrEquities);
        symbols.sort();
        assert_eq!(symbols, vec!["HGRU11".to_string()]);
    }
}
