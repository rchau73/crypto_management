use crate::api_client::CryptoProvider;
use crate::csv_store::AllocationStore;
use crate::domain::repository::HistoryRepo;
use crate::usecases::compute_allocations::compute_allocations;
use std::sync::Arc;

pub struct AllocationsService {
    pub provider: Arc<dyn CryptoProvider>,
    pub repo: Arc<dyn HistoryRepo>,
}

impl AllocationsService {
    pub fn new(provider: Arc<dyn CryptoProvider>, repo: Arc<dyn HistoryRepo>) -> Self {
        Self { provider, repo }
    }

    pub async fn compute_and_record(
        &self,
        api_key: &str,
        current_market: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // fetch cryptos
        let cryptos = self.provider.fetch_latest(api_key).await?;

        // read barca targets from CSV (legacy) — consider migrating to DB
        let store = crate::csv_store::FileCsvStore;
        let barca_targets = store.read_barca_allocations("wallet_barca.csv", current_market)?;

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
