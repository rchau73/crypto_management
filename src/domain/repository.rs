//! Persistence "ports": the traits use cases depend on. `infra::sqlite`
//! implements all of them; tests can swap in fakes. Each trait covers one
//! area so a use case only depends on what it actually uses.

use crate::domain::models::{
    AllocationSnapshot, AssetHistoryRow, BarcaHistoryRow, BarcaTarget, GroupHistoryRow,
    LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PositionSource, RefreshToken, TotalHistoryRow,
    User, WalletPosition,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;

pub type RepoError = Box<dyn std::error::Error + Send + Sync>;
pub type RepoResult<T> = Result<T, RepoError>;

/// Returned (boxed) when an insert/update hits a UNIQUE constraint, so a use
/// case can turn it into a friendly "already exists" error with
/// `error.downcast_ref::<UniqueViolation>()` instead of parsing DB messages.
#[derive(Debug)]
pub struct UniqueViolation {
    /// The database's description of the constraint, e.g.
    /// "UNIQUE constraint failed: users.username".
    pub constraint: String,
}

impl fmt::Display for UniqueViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.constraint)
    }
}

impl std::error::Error for UniqueViolation {}

/// The wallet ledger and the portfolio targets. Grouped in one trait
/// because some writes must touch both tables in a single transaction.
#[async_trait]
pub trait PortfolioRepo: Send + Sync {
    /// One row per position key (the `wallet_allocations_current` view).
    async fn fetch_positions(&self) -> RepoResult<Vec<WalletPosition>>;

    /// The latest ledger row of every source, i.e. `fetch_positions`
    /// before the sources are summed.
    async fn fetch_position_sources(&self) -> RepoResult<Vec<PositionSource>>;

    /// Appends ledger rows in one transaction (all or nothing).
    async fn append_ledger_entries(&self, entries: &[LedgerEntry]) -> RepoResult<()>;

    /// Replaces every portfolio target with `targets` and appends
    /// `new_entries`, in one transaction.
    async fn replace_targets(
        &self,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
        updated_by: Option<i64>,
    ) -> RepoResult<()>;

    /// Adds targets only for keys that have none yet (existing targets are
    /// never overwritten) and appends `new_entries`, in one transaction.
    /// Used by CSV import.
    async fn import_positions(
        &self,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
    ) -> RepoResult<()>;
}

#[async_trait]
pub trait BarcaTargetRepo: Send + Sync {
    async fn fetch_barca_targets(&self, market: &str) -> RepoResult<Vec<BarcaTarget>>;

    /// Replaces every target of `market` with `targets`, in one transaction
    /// (a barca left out is deleted).
    async fn replace_barca_targets(
        &self,
        market: &str,
        targets: &[NewBarcaTarget],
        updated_by: Option<i64>,
    ) -> RepoResult<()>;
}

/// Price-history snapshots, one per "Update Prices" click.
#[async_trait]
pub trait SnapshotRepo: Send + Sync {
    /// Writes the full report plus per-asset/group/barca/total history rows
    /// in one transaction.
    async fn record_snapshot(&self, snapshot: &AllocationSnapshot) -> RepoResult<()>;

    // These return the whole history, oldest first. Fine for a personal
    // dashboard (a few rows per click); add a date range if it ever grows
    // large enough to be slow.
    async fn fetch_asset_history(&self) -> RepoResult<Vec<AssetHistoryRow>>;
    async fn fetch_group_history(&self) -> RepoResult<Vec<GroupHistoryRow>>;
    async fn fetch_barca_history(&self) -> RepoResult<Vec<BarcaHistoryRow>>;
    async fn fetch_total_history(&self) -> RepoResult<Vec<TotalHistoryRow>>;

    /// Price (USD) of every symbol in the most recent snapshot. Empty if
    /// prices were never updated.
    async fn fetch_latest_prices(&self) -> RepoResult<HashMap<String, f64>>;
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub password_hash: &'a str,
    pub role: &'a str,
    pub email: &'a str,
    pub phone: Option<&'a str>,
}

/// A partial update: only the fields that are `Some` change.
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
    /// Returns `false` if no user has this id.
    async fn update_user(&self, id: i64, update: UserUpdate<'_>) -> RepoResult<bool>;
    /// Returns `false` if no user has this id.
    async fn delete_user(&self, id: i64) -> RepoResult<bool>;
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
