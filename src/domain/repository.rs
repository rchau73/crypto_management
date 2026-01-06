use crate::domain::models::{
    AllocationRecord, AssetHistoryRow, AssetSnapshot, BarcaHistoryRow, BarcaSnapshot,
    GroupHistoryRow, GroupSnapshot, TotalSnapshot, WalletAllocation,
};
use async_trait::async_trait;

pub type RepoResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_trait]
pub trait HistoryRepo: Send + Sync {
    async fn insert_asset_snapshot(&self, snap: &AssetSnapshot) -> RepoResult<()>;
    async fn insert_barca_snapshot(&self, snap: &BarcaSnapshot) -> RepoResult<()>;
    async fn insert_total_snapshot(&self, snap: &TotalSnapshot) -> RepoResult<()>;

    async fn fetch_assets(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<AssetHistoryRow>>;
    async fn fetch_barca(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<BarcaHistoryRow>>;
    async fn fetch_groups(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<GroupHistoryRow>>;
    async fn fetch_totals(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<TotalSnapshot>>;

    // Wallet allocations ledger (append-only)
    // Insert a new wallet allocation record (do not delete or update existing rows)
    async fn insert_wallet_allocation(&self, wa: &WalletAllocation) -> RepoResult<()>;
    // Fetch latest/current wallet allocations (one row per symbol representing the most recent entry)
    async fn fetch_current_wallet_allocations(&self) -> RepoResult<Vec<WalletAllocation>>;
    // Fetch audit/history for a given symbol (all rows for symbol ordered by created_at desc)
    #[allow(dead_code)]
    async fn fetch_wallet_allocation_history(
        &self,
        symbol: &str,
    ) -> RepoResult<Vec<WalletAllocation>>;

    // Persist computed allocation payload
    async fn persist_allocation_record(&self, rec: &AllocationRecord) -> RepoResult<()>;

    // Groups history
    async fn insert_group_snapshot(&self, snap: &GroupSnapshot) -> RepoResult<()>;
}
