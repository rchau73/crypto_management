//! Persistence "ports": the traits use cases depend on. `infra::sqlite`
//! implements all of them; tests can swap in fakes. Each trait covers one
//! area so a use case only depends on what it actually uses.
//!
//! Every wallet, target and history method takes the `PortfolioId` it reads
//! or writes, and only ever touches that portfolio's rows. That parameter is
//! what keeps separate wallets from mixing.

use crate::domain::models::{
    AllocationSnapshot, AssetHistoryRow, BarcaHistoryRow, BarcaTarget, GroupHistoryRow,
    LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PortfolioAccess, PortfolioId,
    PortfolioSummary, PositionSource, RecordedActivity, RefreshToken, SourceChange,
    TotalHistoryRow, TradeRecord, TransferRecord, User, WalletPosition,
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

/// Returned (boxed) when a source changed between the read and the write of
/// a `SourceChange` (another trade or import got there first). Nothing is
/// written; the use case reads again and retries.
#[derive(Debug)]
pub struct StaleSource {
    /// e.g. "BTC on Binance".
    pub source: String,
}

impl fmt::Display for StaleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} changed while saving", self.source)
    }
}

impl std::error::Error for StaleSource {}

/// The wallet ledger and the portfolio targets. Grouped in one trait
/// because some writes must touch both tables in a single transaction.
#[async_trait]
pub trait PortfolioRepo: Send + Sync {
    /// One row per position key (the `wallet_allocations_current` view).
    async fn fetch_positions(&self, portfolio: PortfolioId) -> RepoResult<Vec<WalletPosition>>;

    /// The latest ledger row of every source, i.e. `fetch_positions`
    /// before the sources are summed.
    async fn fetch_position_sources(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<Vec<PositionSource>>;

    /// Appends ledger rows in one transaction (all or nothing).
    async fn append_ledger_entries(
        &self,
        portfolio: PortfolioId,
        entries: &[LedgerEntry],
    ) -> RepoResult<()>;

    /// Replaces every target of the portfolio with `targets` and appends
    /// `new_entries`, in one transaction.
    async fn replace_targets(
        &self,
        portfolio: PortfolioId,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
        updated_by: Option<i64>,
    ) -> RepoResult<()>;

    /// Adds targets only for keys that have none yet (existing targets are
    /// never overwritten) and appends `new_entries`, in one transaction.
    /// Used by CSV import.
    async fn import_positions(
        &self,
        portfolio: PortfolioId,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
    ) -> RepoResult<()>;

    /// Logs `trade` and appends the ledger rows it changes (the traded
    /// source, and the cash source if any), in one transaction. Fails with
    /// `StaleSource` and writes nothing if a source no longer holds the
    /// quantity its change was worked out from.
    async fn record_trade(
        &self,
        portfolio: PortfolioId,
        trade: &TradeRecord,
        changes: &[SourceChange],
    ) -> RepoResult<()>;

    /// Logs `transfer` and appends the two ledger rows it changes (the
    /// source it left and the one it reached), in one transaction, with the
    /// same `StaleSource` check as `record_trade`.
    async fn record_transfer(
        &self,
        portfolio: PortfolioId,
        transfer: &TransferRecord,
        changes: &[SourceChange],
    ) -> RepoResult<()>;

    /// How many trades and moves the portfolio has recorded, and the latest
    /// one's time.
    async fn fetch_recorded_activity(&self, portfolio: PortfolioId)
    -> RepoResult<RecordedActivity>;
}

#[async_trait]
pub trait BarcaTargetRepo: Send + Sync {
    async fn fetch_barca_targets(
        &self,
        portfolio: PortfolioId,
        market: &str,
    ) -> RepoResult<Vec<BarcaTarget>>;

    /// Replaces every target of `market` with `targets`, in one transaction
    /// (a barca left out is deleted).
    async fn replace_barca_targets(
        &self,
        portfolio: PortfolioId,
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
    async fn record_snapshot(
        &self,
        portfolio: PortfolioId,
        snapshot: &AllocationSnapshot,
    ) -> RepoResult<()>;

    // These return the portfolio's whole history, oldest first. Fine for a
    // personal dashboard (a few rows per click); add a date range if it ever
    // grows large enough to be slow.
    async fn fetch_asset_history(&self, portfolio: PortfolioId)
    -> RepoResult<Vec<AssetHistoryRow>>;
    async fn fetch_group_history(&self, portfolio: PortfolioId)
    -> RepoResult<Vec<GroupHistoryRow>>;
    async fn fetch_barca_history(&self, portfolio: PortfolioId)
    -> RepoResult<Vec<BarcaHistoryRow>>;
    async fn fetch_total_history(&self, portfolio: PortfolioId)
    -> RepoResult<Vec<TotalHistoryRow>>;

    /// Price (USD) of every symbol in the portfolio's most recent snapshot.
    /// Empty if its prices were never updated.
    async fn fetch_latest_prices(&self, portfolio: PortfolioId)
    -> RepoResult<HashMap<String, f64>>;
}

/// The portfolios themselves and who may open them.
#[async_trait]
pub trait PortfolioDirectoryRepo: Send + Sync {
    /// Every portfolio `user_id` is a member of, by name.
    async fn list_portfolios_for_user(&self, user_id: i64) -> RepoResult<Vec<PortfolioSummary>>;

    /// Every portfolio's id, whoever is a member — for system work such as
    /// the shared price refresh, never to answer a user's request.
    async fn list_all_portfolio_ids(&self) -> RepoResult<Vec<PortfolioId>>;

    /// `user_id`'s access to `portfolio`, or `None` if not a member (or no
    /// such portfolio — callers must not be able to tell the two apart).
    async fn find_access(
        &self,
        portfolio: PortfolioId,
        user_id: i64,
    ) -> RepoResult<Option<PortfolioAccess>>;

    /// Creates a portfolio with `owner_id` as its `manage` member and, if
    /// given, a copy of `copy_barca_from`'s BARCA targets — in one
    /// transaction. A taken name is a `UniqueViolation`.
    async fn create_portfolio(
        &self,
        name: &str,
        owner_id: i64,
        copy_barca_from: Option<PortfolioId>,
    ) -> RepoResult<PortfolioId>;

    /// Returns `false` if there is no such portfolio. A taken name is a
    /// `UniqueViolation`.
    async fn rename_portfolio(&self, portfolio: PortfolioId, name: &str) -> RepoResult<bool>;

    /// Adds `user_id` as a member, or changes the access of one already.
    async fn set_member(
        &self,
        portfolio: PortfolioId,
        user_id: i64,
        access: PortfolioAccess,
    ) -> RepoResult<()>;
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
