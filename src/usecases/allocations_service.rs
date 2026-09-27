use crate::csv_store::AllocationStore;
use crate::domain::market_data::CryptoProvider;
use crate::domain::repository::HistoryRepo;
use crate::usecases::compute_allocations::compute_allocations;
use std::sync::Arc;

pub struct AllocationsService {
    pub provider: Arc<dyn CryptoProvider>,
    pub repo: Arc<dyn HistoryRepo>,
    pub allocation_store: Arc<dyn AllocationStore>,
}

impl AllocationsService {
    pub fn new(
        provider: Arc<dyn CryptoProvider>,
        repo: Arc<dyn HistoryRepo>,
        allocation_store: Arc<dyn AllocationStore>,
    ) -> Self {
        Self {
            provider,
            repo,
            allocation_store,
        }
    }

    const BARCA_TARGETS_PATH: &'static str = "wallet_barca.csv";

    pub async fn compute_and_record(
        &self,
        api_key: &str,
        current_market: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // fetch cryptos
        let cryptos = self.provider.fetch_latest(api_key).await?;

        // read barca targets from CSV (source of truth for target allocations)
        let barca_targets = self
            .allocation_store
            .read_barca_allocations(Self::BARCA_TARGETS_PATH, current_market)?;

        // get current wallet allocations from repo
        let allocs = self.repo.fetch_current_wallet_allocations().await?;

        // compute
        let res = compute_allocations(&allocs, &cryptos, &barca_targets);

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
