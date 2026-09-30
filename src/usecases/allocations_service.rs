//! The "Update Prices" flow: price every position, compute the report and
//! record it in the history.

use crate::domain::market_data::{CryptoProvider, EquityProvider};
use crate::domain::models::{
    AllocationReport, AllocationSnapshot, AssetClass, MarketQuote, WalletPosition,
};
use crate::domain::repository::{BarcaTargetRepo, PortfolioRepo, RepoError, SnapshotRepo};
use crate::usecases::compute_allocations::compute_allocations;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub enum AllocationsError {
    /// CoinMarketCap failed. Crypto is the core of the portfolio, so
    /// without it there is nothing meaningful to show.
    CryptoPrices(RepoError),
    Repo(RepoError),
}

impl fmt::Display for AllocationsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocationsError::CryptoPrices(e) => write!(f, "failed to fetch crypto prices: {e}"),
            AllocationsError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AllocationsError {}

impl From<RepoError> for AllocationsError {
    fn from(e: RepoError) -> Self {
        AllocationsError::Repo(e)
    }
}

/// The price sources, one per asset class.
pub struct MarketProviders {
    pub crypto: Arc<dyn CryptoProvider>,
    pub br_equities: Arc<dyn EquityProvider>,
    pub us_indices: Arc<dyn EquityProvider>,
}

pub struct AllocationsService {
    providers: MarketProviders,
    portfolio: Arc<dyn PortfolioRepo>,
    barca_targets: Arc<dyn BarcaTargetRepo>,
    snapshots: Arc<dyn SnapshotRepo>,
    /// Which BARCA target profile is active ("BullMarket", "BearMarket"...).
    current_market: String,
}

impl AllocationsService {
    pub fn new(
        providers: MarketProviders,
        portfolio: Arc<dyn PortfolioRepo>,
        barca_targets: Arc<dyn BarcaTargetRepo>,
        snapshots: Arc<dyn SnapshotRepo>,
        current_market: String,
    ) -> Self {
        Self {
            providers,
            portfolio,
            barca_targets,
            snapshots,
            current_market,
        }
    }

    /// Fetches fresh prices, computes the report and records it as one
    /// history snapshot.
    ///
    /// Crypto prices are required. A Brazilian/US equity provider failing
    /// only leaves those assets out of this round (logged), since they are
    /// optional extras the user may not have API keys for.
    pub async fn refresh_prices(&self) -> Result<AllocationReport, AllocationsError> {
        let positions = self.portfolio.fetch_positions().await?;
        let quotes = self.fetch_all_quotes(&positions).await?;

        let barca_targets: HashMap<String, f64> = self
            .barca_targets
            .fetch_barca_targets(&self.current_market)
            .await?
            .into_iter()
            .map(|t| (t.barca, t.target_percent))
            .collect();

        let report = compute_allocations(&positions, &quotes, &barca_targets);

        self.snapshots
            .record_snapshot(&AllocationSnapshot {
                timestamp: chrono::Utc::now().to_rfc3339(),
                report: report.clone(),
            })
            .await?;

        tracing::info!(
            assets = report.per_asset.len(),
            groups = report.per_group.len(),
            total_value = report.total_value,
            market = %self.current_market,
            "Computed and recorded allocation snapshot"
        );
        Ok(report)
    }

    async fn fetch_all_quotes(
        &self,
        positions: &[WalletPosition],
    ) -> Result<Vec<MarketQuote>, AllocationsError> {
        let mut quotes = self
            .providers
            .crypto
            .fetch_latest()
            .await
            .map_err(AllocationsError::CryptoPrices)?;

        let optional_sources = [
            (AssetClass::BrEquities, &self.providers.br_equities),
            (AssetClass::UsIndices, &self.providers.us_indices),
        ];
        for (class, provider) in optional_sources {
            let symbols = symbols_of_class(positions, class);
            if symbols.is_empty() {
                continue;
            }
            match provider.fetch_quotes(&symbols).await {
                Ok(more) => quotes.extend(more),
                Err(e) => tracing::error!(
                    error = %e,
                    asset_class = class.as_str(),
                    ?symbols,
                    "Failed to fetch quotes — these assets are left out of this refresh"
                ),
            }
        }
        Ok(quotes)
    }
}

/// Distinct symbols of one asset class, sorted.
fn symbols_of_class(positions: &[WalletPosition], class: AssetClass) -> Vec<String> {
    positions
        .iter()
        .filter(|p| p.asset_class == class.as_str())
        .map(|p| p.symbol.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::market_data::FakeProvider;
    use crate::domain::models::{LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PositionKey};
    use crate::infra::sqlite::{SqliteRepo, test_repo};

    async fn seed(repo: &SqliteRepo, symbol: &str, asset_class: &str, qty: f64, target: f64) {
        let entry = LedgerEntry {
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            asset_class: asset_class.to_string(),
            current_quantity: qty,
            last_price: None,
            notes: None,
        };
        let target = NewPortfolioTarget {
            key: PositionKey {
                symbol: symbol.to_string(),
                group_name: "Core".to_string(),
                barca: "Base".to_string(),
                asset_class: asset_class.to_string(),
            },
            target_percent: target,
        };
        repo.import_positions(&[target], &[entry]).await.unwrap();
    }

    async fn set_barca_target(repo: &SqliteRepo, market: &str, target_percent: f64) {
        repo.replace_barca_targets(
            market,
            &[NewBarcaTarget {
                barca: "Base".to_string(),
                target_percent,
            }],
            None,
        )
        .await
        .unwrap();
    }

    fn service(repo: Arc<SqliteRepo>, providers: MarketProviders) -> AllocationsService {
        AllocationsService::new(
            providers,
            repo.clone(),
            repo.clone(),
            repo,
            "BullMarket".to_string(),
        )
    }

    fn providers(crypto: FakeProvider, br: FakeProvider) -> MarketProviders {
        MarketProviders {
            crypto: Arc::new(crypto),
            br_equities: Arc::new(br),
            us_indices: Arc::new(FakeProvider::empty()),
        }
    }

    #[tokio::test]
    async fn merges_crypto_and_equity_prices_and_records_a_snapshot() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        seed(&repo, "HGRU11", "br-equities", 10.0, 50.0).await;
        set_barca_target(&repo, "BullMarket", 100.0).await;

        let report = service(
            repo.clone(),
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::with_prices(&[("HGRU11", 100.0)]),
            ),
        )
        .refresh_prices()
        .await
        .unwrap();

        assert_eq!(report.total_value, 1010.0);
        assert_eq!(report.per_asset.len(), 2);
        let totals = repo.fetch_total_history().await.unwrap();
        assert_eq!(totals.len(), 1, "one snapshot recorded");
        assert_eq!(totals[0].total_value, Some(1010.0));
    }

    #[tokio::test]
    async fn an_equity_provider_failure_does_not_abort_the_refresh() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        seed(&repo, "HGRU11", "br-equities", 10.0, 0.0).await;

        let report = service(
            repo,
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::failing(),
            ),
        )
        .refresh_prices()
        .await
        .unwrap();

        let symbols: Vec<_> = report.per_asset.iter().map(|a| a.symbol.as_str()).collect();
        assert_eq!(symbols, ["BTC"]);
    }

    #[tokio::test]
    async fn a_crypto_provider_failure_is_a_typed_error_and_records_nothing() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;

        let err = service(
            repo.clone(),
            providers(FakeProvider::failing(), FakeProvider::empty()),
        )
        .refresh_prices()
        .await
        .unwrap_err();

        assert!(matches!(err, AllocationsError::CryptoPrices(_)));
        assert!(repo.fetch_total_history().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn uses_only_the_current_markets_barca_targets() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        set_barca_target(&repo, "BullMarket", 100.0).await;
        set_barca_target(&repo, "BearMarket", 40.0).await;

        let report = service(
            repo,
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::empty(),
            ),
        )
        .refresh_prices()
        .await
        .unwrap();

        assert_eq!(report.per_barca[0].target_percent, 100.0);
    }

    #[test]
    fn symbols_of_class_filters_dedupes_and_sorts() {
        let position = |symbol: &str, class: &str| WalletPosition {
            symbol: symbol.to_string(),
            group_name: None,
            barca: None,
            asset_class: class.to_string(),
            target_percent: 0.0,
            current_quantity: 1.0,
            last_price: None,
            notes: None,
            source_count: 1,
        };
        let positions = vec![
            position("XPML11", "br-equities"),
            position("HGRU11", "br-equities"),
            position("HGRU11", "br-equities"),
            position("BTC", "crypto"),
        ];
        assert_eq!(
            symbols_of_class(&positions, AssetClass::BrEquities),
            ["HGRU11", "XPML11"]
        );
    }
}
