use crate::domain::models::{
    AllocationRecord, AssetHistoryRow, AssetSnapshot, BarcaHistoryRow, BarcaSnapshot, BarcaTarget,
    GroupHistoryRow, GroupSnapshot, PortfolioTarget, RefreshToken, TotalSnapshot, User,
    WalletAllocation,
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
    // Insert a new wallet allocation record (do not delete or update existing rows).
    // No production caller needs a single-row insert anymore (CSV import
    // batches via bulk_insert_wallet_allocations below) — kept for tests
    // that need to set up one specific row at a time.
    #[allow(dead_code)]
    async fn insert_wallet_allocation(&self, wa: &WalletAllocation) -> RepoResult<()>;
    // Insert several rows as one atomic transaction — the "Save all" bulk
    // portfolio edit either fully lands or fully fails, never half-applies.
    async fn bulk_insert_wallet_allocations(&self, rows: &[WalletAllocation]) -> RepoResult<()>;
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

pub struct NewUser<'a> {
    pub username: &'a str,
    pub password_hash: &'a str,
    pub role: &'a str,
    pub email: &'a str,
    pub phone: Option<&'a str>,
}

// Every field is applied only when Some — a partial update, not a full
// overwrite, so an admin can change just the role, or just reset a password,
// without having to resend everything else.
#[derive(Default)]
pub struct UserUpdate<'a> {
    pub role: Option<&'a str>,
    pub password_hash: Option<&'a str>,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
}

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn find_user_by_username(&self, username: &str) -> RepoResult<Option<User>>;
    async fn find_user_by_id(&self, id: i64) -> RepoResult<Option<User>>;
    async fn list_users(&self) -> RepoResult<Vec<User>>;
    async fn count_users(&self) -> RepoResult<i64>;
    async fn create_user(&self, new_user: NewUser<'_>) -> RepoResult<User>;
    async fn update_user(&self, id: i64, update: UserUpdate<'_>) -> RepoResult<()>;
    async fn delete_user(&self, id: i64) -> RepoResult<()>;
}

#[async_trait]
pub trait RefreshTokenRepo: Send + Sync {
    async fn store_refresh_token(
        &self,
        user_id: i64,
        token_hash: &str,
        expires_at: &str,
    ) -> RepoResult<()>;
    async fn find_refresh_token(&self, token_hash: &str) -> RepoResult<Option<RefreshToken>>;
    async fn revoke_refresh_token(&self, token_hash: &str) -> RepoResult<()>;
}

pub struct BarcaTargetInput<'a> {
    pub barca: &'a str,
    pub target_percent: f64,
}

#[async_trait]
pub trait BarcaTargetRepo: Send + Sync {
    async fn fetch_barca_targets(&self, market: &str) -> RepoResult<Vec<BarcaTarget>>;
    // Replaces every target row for this market with exactly the given set,
    // as one atomic transaction — a barca left out of `targets` is deleted,
    // matching "editable table + Save all" semantics (the UI always submits
    // the full current state, not a diff).
    async fn replace_barca_targets(
        &self,
        market: &str,
        targets: &[BarcaTargetInput<'_>],
        updated_by: Option<i64>,
    ) -> RepoResult<()>;
}

pub struct PortfolioTargetInput<'a> {
    pub symbol: &'a str,
    pub group_name: &'a str,
    pub barca: &'a str,
    pub asset_class: &'a str,
    pub target_percent: f64,
}

#[async_trait]
pub trait PortfolioTargetRepo: Send + Sync {
    #[allow(dead_code)]
    async fn fetch_portfolio_targets(&self) -> RepoResult<Vec<PortfolioTarget>>;
    // Replaces the whole table with exactly the given set, as one atomic
    // transaction — same "editable table + Save all" semantics as
    // replace_barca_targets (a row left out of `targets` is deleted).
    async fn replace_portfolio_targets(
        &self,
        targets: &[PortfolioTargetInput<'_>],
        updated_by: Option<i64>,
    ) -> RepoResult<()>;
    // Inserts a target only for a (symbol, group, barca, asset_class) that
    // doesn't already have one — used by CSV import to seed a brand-new
    // asset's target without ever overwriting a manager-set value for an
    // asset that already exists ("local distribution always overcomes the
    // CSV file entries").
    async fn seed_portfolio_targets_if_absent(
        &self,
        targets: &[PortfolioTargetInput<'_>],
    ) -> RepoResult<()>;
}
