//! SQLite implementation of every repository port in `domain::repository`.
//!
//! Every query on a wallet, target or history table filters (or inserts)
//! `portfolio_id`. The column is NOT NULL with no default, so an INSERT that
//! forgets it fails instead of landing in another portfolio.

use crate::domain::models::{
    AllocationSnapshot, AssetHistoryRow, BarcaHistoryRow, BarcaTarget, GroupHistoryRow,
    LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PortfolioAccess, PortfolioId,
    PortfolioSummary, PositionSource, RecordedActivity, RefreshToken, SourceChange,
    TotalHistoryRow, TradeRecord, TransferRecord, User, WalletPosition,
};
use crate::domain::repository::{
    BarcaTargetRepo, NewUser, PortfolioDirectoryRepo, PortfolioRepo, RefreshTokenRepo, RepoError,
    RepoResult, SnapshotRepo, StaleSource, UniqueViolation, UserRepo, UserUpdate,
};
use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

pub struct SqliteRepo {
    pool: SqlitePool,
}

impl SqliteRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// For tests that check a table no port reads (e.g. the trades log).
    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

/// Converts a UNIQUE-constraint failure into the typed `UniqueViolation`,
/// so use cases can react to it without parsing error strings.
fn map_db_error(e: sqlx::Error) -> RepoError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Box::new(UniqueViolation {
                constraint: db.message().to_string(),
            });
        }
    }
    Box::new(e)
}

async fn insert_ledger_entries(
    tx: &mut Transaction<'_, Sqlite>,
    portfolio: PortfolioId,
    entries: &[LedgerEntry],
) -> RepoResult<()> {
    for entry in entries {
        sqlx::query(
            "INSERT INTO wallet_allocations \
             (portfolio_id, symbol, group_name, barca, asset_class, current_quantity, \
              last_price, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(portfolio)
        .bind(&entry.symbol)
        .bind(&entry.group_name)
        .bind(&entry.barca)
        .bind(&entry.asset_class)
        .bind(entry.current_quantity)
        .bind(entry.last_price)
        .bind(&entry.notes)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Appends each change's ledger row after checking its source still holds
/// the quantity the change was worked out from (`StaleSource` otherwise).
///
/// Call it only once the transaction has written something: SQLite then
/// holds the write lock, so no other writer can slip in between the check
/// and the insert.
async fn apply_source_changes(
    tx: &mut Transaction<'_, Sqlite>,
    portfolio: PortfolioId,
    changes: &[SourceChange],
) -> RepoResult<()> {
    for change in changes {
        let entry = &change.entry;
        // The source's latest row, by the view's rule (migration 0013).
        let now: Option<f64> = sqlx::query_scalar(
            "SELECT COALESCE(current_quantity, 0.0) FROM wallet_allocations \
             WHERE portfolio_id = ?1 AND symbol = ?2 AND group_name IS ?3 AND barca IS ?4 \
               AND asset_class = ?5 AND COALESCE(notes, '') = COALESCE(?6, '') \
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(portfolio)
        .bind(&entry.symbol)
        .bind(&entry.group_name)
        .bind(&entry.barca)
        .bind(&entry.asset_class)
        .bind(&entry.notes)
        .fetch_optional(&mut **tx)
        .await?;
        // Exact comparison on purpose: `before` was read from this column.
        if now.unwrap_or(0.0) != change.before {
            return Err(Box::new(StaleSource {
                source: format!(
                    "{} on {}",
                    entry.symbol,
                    entry.notes.as_deref().unwrap_or("the unnamed source")
                ),
            }));
        }
    }
    let entries: Vec<LedgerEntry> = changes.iter().map(|c| c.entry.clone()).collect();
    insert_ledger_entries(tx, portfolio, &entries).await
}

#[async_trait]
impl PortfolioRepo for SqliteRepo {
    async fn fetch_positions(&self, portfolio: PortfolioId) -> RepoResult<Vec<WalletPosition>> {
        let rows = sqlx::query_as::<_, WalletPosition>(
            "SELECT symbol, group_name, barca, asset_class, target_percent, current_quantity, \
                    last_price, notes, source_count \
             FROM wallet_allocations_current \
             WHERE portfolio_id = ?1 \
             ORDER BY symbol, group_name, barca, asset_class",
        )
        .bind(portfolio)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_position_sources(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<Vec<PositionSource>> {
        // Same "latest row per source" rule as the wallet_allocations_current
        // view (migration 0013), without the SUM.
        let rows = sqlx::query_as::<_, PositionSource>(
            "SELECT symbol, group_name, barca, asset_class, \
                    COALESCE(current_quantity, 0.0) AS current_quantity, notes \
             FROM ( \
                 SELECT *, ROW_NUMBER() OVER ( \
                     PARTITION BY symbol, group_name, barca, asset_class, COALESCE(notes, '') \
                     ORDER BY created_at DESC, id DESC \
                 ) AS rn \
                 FROM wallet_allocations \
                 WHERE portfolio_id = ?1 \
             ) \
             WHERE rn = 1 \
             ORDER BY symbol, group_name, barca, asset_class, notes",
        )
        .bind(portfolio)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn append_ledger_entries(
        &self,
        portfolio: PortfolioId,
        entries: &[LedgerEntry],
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        insert_ledger_entries(&mut tx, portfolio, entries).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn replace_targets(
        &self,
        portfolio: PortfolioId,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
        updated_by: Option<i64>,
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM portfolio_targets WHERE portfolio_id = ?1")
            .bind(portfolio)
            .execute(&mut *tx)
            .await?;
        for target in targets {
            sqlx::query(
                "INSERT INTO portfolio_targets \
                 (portfolio_id, symbol, group_name, barca, asset_class, target_percent, \
                  updated_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .bind(portfolio)
            .bind(&target.key.symbol)
            .bind(&target.key.group_name)
            .bind(&target.key.barca)
            .bind(&target.key.asset_class)
            .bind(target.target_percent)
            .bind(updated_by)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        insert_ledger_entries(&mut tx, portfolio, new_entries).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn import_positions(
        &self,
        portfolio: PortfolioId,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        for target in targets {
            sqlx::query(
                "INSERT OR IGNORE INTO portfolio_targets \
                 (portfolio_id, symbol, group_name, barca, asset_class, target_percent) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(portfolio)
            .bind(&target.key.symbol)
            .bind(&target.key.group_name)
            .bind(&target.key.barca)
            .bind(&target.key.asset_class)
            .bind(target.target_percent)
            .execute(&mut *tx)
            .await?;
        }
        insert_ledger_entries(&mut tx, portfolio, new_entries).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn record_trade(
        &self,
        portfolio: PortfolioId,
        trade: &TradeRecord,
        changes: &[SourceChange],
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        // The log row goes first: it takes the write lock before the
        // sources are checked (see apply_source_changes).
        sqlx::query(
            "INSERT INTO trades \
             (portfolio_id, side, symbol, group_name, barca, asset_class, source, quantity, \
              price, cash_symbol, cash_source, cash_quantity, recorded_by) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )
        .bind(portfolio)
        .bind(trade.side.as_str())
        .bind(&trade.symbol)
        .bind(&trade.group_name)
        .bind(&trade.barca)
        .bind(&trade.asset_class)
        .bind(&trade.source)
        .bind(trade.quantity)
        .bind(trade.price)
        .bind(&trade.cash_symbol)
        .bind(&trade.cash_source)
        .bind(trade.cash_quantity)
        .bind(trade.recorded_by)
        .execute(&mut *tx)
        .await?;
        apply_source_changes(&mut tx, portfolio, changes).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn record_transfer(
        &self,
        portfolio: PortfolioId,
        transfer: &TransferRecord,
        changes: &[SourceChange],
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        // The log row goes first: it takes the write lock before the
        // sources are checked (see apply_source_changes).
        sqlx::query(
            "INSERT INTO transfers \
             (portfolio_id, symbol, asset_class, from_group_name, from_barca, from_source, \
              to_group_name, to_barca, to_source, quantity_sent, quantity_received, recorded_by) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .bind(portfolio)
        .bind(&transfer.symbol)
        .bind(&transfer.asset_class)
        .bind(&transfer.from_group_name)
        .bind(&transfer.from_barca)
        .bind(&transfer.from_source)
        .bind(&transfer.to_group_name)
        .bind(&transfer.to_barca)
        .bind(&transfer.to_source)
        .bind(transfer.quantity_sent)
        .bind(transfer.quantity_received)
        .bind(transfer.recorded_by)
        .execute(&mut *tx)
        .await?;
        apply_source_changes(&mut tx, portfolio, changes).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn fetch_recorded_activity(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<RecordedActivity> {
        let (trades, transfers, latest_at): (i64, i64, Option<String>) = sqlx::query_as(
            "SELECT \
                 (SELECT COUNT(*) FROM trades WHERE portfolio_id = ?1), \
                 (SELECT COUNT(*) FROM transfers WHERE portfolio_id = ?1), \
                 (SELECT MAX(created_at) FROM ( \
                     SELECT created_at FROM trades WHERE portfolio_id = ?1 \
                     UNION ALL \
                     SELECT created_at FROM transfers WHERE portfolio_id = ?1))",
        )
        .bind(portfolio)
        .fetch_one(&self.pool)
        .await?;
        Ok(RecordedActivity {
            trades,
            transfers,
            latest_at,
        })
    }
}

#[async_trait]
impl BarcaTargetRepo for SqliteRepo {
    async fn fetch_barca_targets(
        &self,
        portfolio: PortfolioId,
        market: &str,
    ) -> RepoResult<Vec<BarcaTarget>> {
        let rows = sqlx::query_as::<_, BarcaTarget>(
            "SELECT id, market, barca, target_percent, updated_by, created_at, updated_at \
             FROM barca_targets WHERE portfolio_id = ?1 AND market = ?2 ORDER BY barca ASC",
        )
        .bind(portfolio)
        .bind(market)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn replace_barca_targets(
        &self,
        portfolio: PortfolioId,
        market: &str,
        targets: &[NewBarcaTarget],
        updated_by: Option<i64>,
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM barca_targets WHERE portfolio_id = ?1 AND market = ?2")
            .bind(portfolio)
            .bind(market)
            .execute(&mut *tx)
            .await?;
        for target in targets {
            sqlx::query(
                "INSERT INTO barca_targets \
                 (portfolio_id, market, barca, target_percent, updated_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(portfolio)
            .bind(market)
            .bind(&target.barca)
            .bind(target.target_percent)
            .bind(updated_by)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl SnapshotRepo for SqliteRepo {
    async fn record_snapshot(
        &self,
        portfolio: PortfolioId,
        snapshot: &AllocationSnapshot,
    ) -> RepoResult<()> {
        let ts = &snapshot.timestamp;
        let report = &snapshot.report;
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO allocations (portfolio_id, computed_at, payload) VALUES (?1, ?2, ?3)",
        )
        .bind(portfolio)
        .bind(ts)
        .bind(serde_json::to_string(report)?)
        .execute(&mut *tx)
        .await?;

        // history_assets is UNIQUE(portfolio_id, timestamp, symbol,
        // group_name, barca), so a symbol held in two groups gets one row per
        // group. `OR IGNORE` only guards against a repeated snapshot.
        for a in &report.per_asset {
            sqlx::query(
                "INSERT OR IGNORE INTO history_assets \
                 (portfolio_id, timestamp, symbol, group_name, barca, price, current_quantity, \
                  value, target_percent, current_percent, extra) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .bind(portfolio)
            .bind(ts)
            .bind(&a.symbol)
            .bind(&a.group)
            .bind(&a.barca)
            .bind(a.price)
            .bind(a.current_quantity)
            .bind(a.value)
            .bind(a.target_percent)
            .bind(a.current_percent)
            .bind(snapshot.asset_notes.get(&a.symbol))
            .execute(&mut *tx)
            .await?;
        }

        for g in &report.per_group {
            sqlx::query(
                "INSERT OR IGNORE INTO history_groups \
                 (portfolio_id, timestamp, group_name, value, current_percent, target_percent) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(portfolio)
            .bind(ts)
            .bind(&g.group)
            .bind(g.value)
            .bind(g.current_percent)
            .bind(g.target_percent)
            .execute(&mut *tx)
            .await?;
        }

        for b in &report.per_barca {
            sqlx::query(
                "INSERT OR IGNORE INTO history_barca \
                 (portfolio_id, timestamp, barca, value, current_percent, target_percent) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(portfolio)
            .bind(ts)
            .bind(&b.barca)
            .bind(b.value)
            .bind(b.current_percent)
            .bind(b.target_percent)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT OR REPLACE INTO history_totals (portfolio_id, timestamp, total_value) \
             VALUES (?1, ?2, ?3)",
        )
        .bind(portfolio)
        .bind(ts)
        .bind(report.total_value)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn fetch_asset_history(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<Vec<AssetHistoryRow>> {
        let rows = sqlx::query_as(
            "SELECT timestamp, symbol, group_name, barca, price, current_quantity, value, \
                    target_percent, current_percent, deviation_percent, value_deviation \
             FROM asset_variance_history WHERE portfolio_id = ?1 \
             ORDER BY timestamp, symbol, group_name, barca",
        )
        .bind(portfolio)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_group_history(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<Vec<GroupHistoryRow>> {
        let rows = sqlx::query_as(
            "SELECT timestamp, group_name, value, current_percent, target_percent, deviation_percent \
             FROM group_variance_history WHERE portfolio_id = ?1 ORDER BY timestamp, group_name",
        )
        .bind(portfolio)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_barca_history(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<Vec<BarcaHistoryRow>> {
        let rows = sqlx::query_as(
            "SELECT timestamp, barca, value, current_percent, target_percent, deviation_percent \
             FROM barca_variance_history WHERE portfolio_id = ?1 ORDER BY timestamp, barca",
        )
        .bind(portfolio)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_total_history(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<Vec<TotalHistoryRow>> {
        let rows = sqlx::query_as(
            "SELECT timestamp, total_value FROM history_totals \
             WHERE portfolio_id = ?1 ORDER BY timestamp",
        )
        .bind(portfolio)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_latest_prices(
        &self,
        portfolio: PortfolioId,
    ) -> RepoResult<HashMap<String, f64>> {
        // Timestamps are RFC 3339 in UTC, so MAX() is the most recent one.
        let rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT symbol, price FROM history_assets \
             WHERE portfolio_id = ?1 \
               AND timestamp = (SELECT MAX(timestamp) FROM history_totals WHERE portfolio_id = ?1) \
               AND price IS NOT NULL",
        )
        .bind(portfolio)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().collect())
    }
}

#[async_trait]
impl PortfolioDirectoryRepo for SqliteRepo {
    async fn list_portfolios_for_user(&self, user_id: i64) -> RepoResult<Vec<PortfolioSummary>> {
        let rows: Vec<(PortfolioId, String, String)> = sqlx::query_as(
            "SELECT p.id, p.name, m.access FROM portfolios p \
             JOIN portfolio_members m ON m.portfolio_id = p.id \
             WHERE m.user_id = ?1 ORDER BY p.name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|(id, name, access)| {
                Ok(PortfolioSummary {
                    id,
                    name,
                    access: parse_access(&access)?,
                })
            })
            .collect()
    }

    async fn list_all_portfolio_ids(&self) -> RepoResult<Vec<PortfolioId>> {
        Ok(sqlx::query_scalar("SELECT id FROM portfolios ORDER BY id")
            .fetch_all(&self.pool)
            .await?)
    }

    async fn find_access(
        &self,
        portfolio: PortfolioId,
        user_id: i64,
    ) -> RepoResult<Option<PortfolioAccess>> {
        let access: Option<String> = sqlx::query_scalar(
            "SELECT access FROM portfolio_members WHERE portfolio_id = ?1 AND user_id = ?2",
        )
        .bind(portfolio)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        access.as_deref().map(parse_access).transpose()
    }

    async fn create_portfolio(
        &self,
        name: &str,
        owner_id: i64,
        copy_barca_from: Option<PortfolioId>,
    ) -> RepoResult<PortfolioId> {
        let mut tx = self.pool.begin().await?;
        let id: PortfolioId =
            sqlx::query_scalar("INSERT INTO portfolios (name) VALUES (?1) RETURNING id")
                .bind(name)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_db_error)?;
        sqlx::query(
            "INSERT INTO portfolio_members (portfolio_id, user_id, access) VALUES (?1, ?2, 'manage')",
        )
        .bind(id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;
        if let Some(source) = copy_barca_from {
            sqlx::query(
                "INSERT INTO barca_targets (portfolio_id, market, barca, target_percent, updated_by) \
                 SELECT ?1, market, barca, target_percent, ?2 FROM barca_targets \
                 WHERE portfolio_id = ?3",
            )
            .bind(id)
            .bind(owner_id)
            .bind(source)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    async fn rename_portfolio(&self, portfolio: PortfolioId, name: &str) -> RepoResult<bool> {
        let result = sqlx::query("UPDATE portfolios SET name = ?1 WHERE id = ?2")
            .bind(name)
            .bind(portfolio)
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_member(
        &self,
        portfolio: PortfolioId,
        user_id: i64,
        access: PortfolioAccess,
    ) -> RepoResult<()> {
        sqlx::query(
            "INSERT INTO portfolio_members (portfolio_id, user_id, access) VALUES (?1, ?2, ?3) \
             ON CONFLICT (portfolio_id, user_id) DO UPDATE SET access = excluded.access",
        )
        .bind(portfolio)
        .bind(user_id)
        .bind(access.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// The CHECK constraint only allows these two, so a failure here means the
/// database was edited by hand.
fn parse_access(raw: &str) -> RepoResult<PortfolioAccess> {
    PortfolioAccess::parse(raw).ok_or_else(|| format!("unknown portfolio access {raw:?}").into())
}

#[async_trait]
impl UserRepo for SqliteRepo {
    async fn find_user_by_username(&self, username: &str) -> RepoResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;
        Ok(user)
    }

    async fn find_user_by_id(&self, id: i64) -> RepoResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(user)
    }

    async fn list_users(&self) -> RepoResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY username ASC")
            .fetch_all(&self.pool)
            .await?;
        Ok(users)
    }

    async fn count_users(&self) -> RepoResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    async fn create_user(&self, new_user: NewUser<'_>) -> RepoResult<User> {
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, password_hash, role, email, phone) \
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING *",
        )
        .bind(new_user.username)
        .bind(new_user.password_hash)
        .bind(new_user.role)
        .bind(new_user.email)
        .bind(new_user.phone)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(user)
    }

    async fn update_user(&self, id: i64, update: UserUpdate<'_>) -> RepoResult<bool> {
        let result = sqlx::query(
            "UPDATE users SET \
                role = COALESCE(?1, role), \
                password_hash = COALESCE(?2, password_hash), \
                email = COALESCE(?3, email), \
                phone = COALESCE(?4, phone), \
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
             WHERE id = ?5",
        )
        .bind(update.role)
        .bind(update.password_hash)
        .bind(update.email)
        .bind(update.phone)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_user(&self, id: i64) -> RepoResult<bool> {
        // Other tables point at users(id) and foreign keys are enforced, so
        // clear those references first — all in one transaction.
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM portfolio_members WHERE user_id = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE portfolio_targets SET updated_by = NULL WHERE updated_by = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE barca_targets SET updated_by = NULL WHERE updated_by = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        // The trades and moves logs keep each entry, without who recorded it.
        sqlx::query("UPDATE trades SET recorded_by = NULL WHERE recorded_by = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE transfers SET recorded_by = NULL WHERE recorded_by = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let result = sqlx::query("DELETE FROM users WHERE id = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait]
impl RefreshTokenRepo for SqliteRepo {
    async fn store_refresh_token(
        &self,
        user_id: i64,
        token_hash: &str,
        expires_at: &str,
    ) -> RepoResult<()> {
        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES (?1, ?2, ?3)",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_refresh_token(&self, token_hash: &str) -> RepoResult<Option<RefreshToken>> {
        let token =
            sqlx::query_as::<_, RefreshToken>("SELECT * FROM refresh_tokens WHERE token_hash = ?1")
                .bind(token_hash)
                .fetch_optional(&self.pool)
                .await?;
        Ok(token)
    }

    async fn revoke_refresh_token(&self, token_hash: &str) -> RepoResult<()> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
             WHERE token_hash = ?1",
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// Test-only reads that production code never needs.
#[cfg(test)]
impl SqliteRepo {
    pub async fn count_ledger_rows(&self, symbol: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM wallet_allocations WHERE symbol = ?1")
            .bind(symbol)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    pub async fn target_of(&self, symbol: &str) -> Option<f64> {
        sqlx::query_scalar("SELECT target_percent FROM portfolio_targets WHERE symbol = ?1")
            .bind(symbol)
            .fetch_optional(&self.pool)
            .await
            .unwrap()
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use crate::domain::models::PositionKey;

    pub fn entry(symbol: &str, quantity: f64, notes: Option<&str>) -> LedgerEntry {
        LedgerEntry {
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            asset_class: "crypto".to_string(),
            current_quantity: quantity,
            last_price: None,
            notes: notes.map(str::to_string),
        }
    }

    pub fn target(symbol: &str, target_percent: f64) -> NewPortfolioTarget {
        NewPortfolioTarget {
            key: PositionKey {
                symbol: symbol.to_string(),
                group_name: "Core".to_string(),
                barca: "Base".to_string(),
                asset_class: "crypto".to_string(),
            },
            target_percent,
        }
    }

    pub fn barca(name: &str, target_percent: f64) -> NewBarcaTarget {
        NewBarcaTarget {
            barca: name.to_string(),
            target_percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{barca, entry, target};
    use super::*;
    use crate::domain::models::{AllocationReport, AssetAllocation};
    use crate::infra::sqlite::test_repo;

    /// These tests are about one wallet; isolation between portfolios is
    /// tested in `infra::sqlite::repo` and `tests::integration`.
    const PORTFOLIO: PortfolioId = PortfolioId::FIRST;

    fn new_user<'a>(username: &'a str, email: &'a str) -> NewUser<'a> {
        NewUser {
            username,
            password_hash: "hash",
            role: "user",
            email,
            phone: None,
        }
    }

    // --- positions ---------------------------------------------------------

    #[tokio::test]
    async fn positions_sum_the_latest_quantity_of_each_source_and_read_the_target() {
        let repo = test_repo().await;
        repo.append_ledger_entries(
            PORTFOLIO,
            &[
                entry("BTC", 1.0, Some("Ledger")),
                entry("BTC", 0.5, Some("Binance")),
                entry("BTC", 2.0, Some("Ledger")), // supersedes Ledger's 1.0
            ],
        )
        .await
        .unwrap();
        repo.replace_targets(PORTFOLIO, &[target("BTC", 40.0)], &[], None)
            .await
            .unwrap();

        let positions = repo.fetch_positions(PORTFOLIO).await.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].current_quantity, 2.5);
        assert_eq!(positions[0].target_percent, 40.0);
        assert_eq!(positions[0].source_count, 2);
    }

    #[tokio::test]
    async fn a_null_notes_source_is_counted_even_though_group_concat_skips_it() {
        // Regression: the UI used to detect "single source" by looking for
        // " | " in the joined notes, which a NULL-notes source never adds.
        let repo = test_repo().await;
        repo.append_ledger_entries(
            PORTFOLIO,
            &[entry("BTC", 1.0, None), entry("BTC", 2.0, Some("Binance"))],
        )
        .await
        .unwrap();

        let position = &repo.fetch_positions(PORTFOLIO).await.unwrap()[0];
        assert_eq!(position.notes.as_deref(), Some("Binance"));
        assert_eq!(position.source_count, 2);
    }

    #[tokio::test]
    async fn a_position_without_a_target_defaults_to_zero() {
        let repo = test_repo().await;
        repo.append_ledger_entries(PORTFOLIO, &[entry("SOL", 1.0, None)])
            .await
            .unwrap();
        assert_eq!(
            repo.fetch_positions(PORTFOLIO).await.unwrap()[0].target_percent,
            0.0
        );
    }

    #[tokio::test]
    async fn the_same_symbol_in_two_asset_classes_stays_two_positions() {
        let repo = test_repo().await;
        let mut us = entry("IVV", 5.0, None);
        us.asset_class = "us-indices".to_string();
        repo.append_ledger_entries(PORTFOLIO, &[us, entry("IVV", 1.0, None)])
            .await
            .unwrap();
        assert_eq!(repo.fetch_positions(PORTFOLIO).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn replace_targets_replaces_the_whole_set_and_appends_new_entries() {
        let repo = test_repo().await;
        repo.replace_targets(
            PORTFOLIO,
            &[target("BTC", 60.0), target("ETH", 40.0)],
            &[],
            None,
        )
        .await
        .unwrap();
        repo.replace_targets(
            PORTFOLIO,
            &[target("BTC", 100.0)],
            &[entry("BTC", 1.0, None)],
            None,
        )
        .await
        .unwrap();

        assert_eq!(repo.target_of("BTC").await, Some(100.0));
        assert_eq!(
            repo.target_of("ETH").await,
            None,
            "left-out rows are deleted"
        );
        assert_eq!(repo.count_ledger_rows("BTC").await, 1);
    }

    #[tokio::test]
    async fn replace_targets_is_all_or_nothing() {
        let repo = test_repo().await;
        repo.replace_targets(PORTFOLIO, &[target("BTC", 100.0)], &[], None)
            .await
            .unwrap();

        // A duplicate key fails mid-transaction: neither the delete, nor
        // the new targets, nor the ledger entry may be kept.
        let err = repo
            .replace_targets(
                PORTFOLIO,
                &[target("ETH", 50.0), target("ETH", 50.0)],
                &[entry("ETH", 1.0, None)],
                None,
            )
            .await
            .unwrap_err();

        assert!(err.downcast_ref::<UniqueViolation>().is_some());
        assert_eq!(repo.target_of("BTC").await, Some(100.0));
        assert_eq!(repo.count_ledger_rows("ETH").await, 0);
    }

    #[tokio::test]
    async fn import_positions_never_overwrites_an_existing_target() {
        let repo = test_repo().await;
        repo.replace_targets(PORTFOLIO, &[target("BTC", 60.0)], &[], None)
            .await
            .unwrap();

        repo.import_positions(PORTFOLIO, &[target("BTC", 99.0), target("ETH", 40.0)], &[])
            .await
            .unwrap();

        assert_eq!(repo.target_of("BTC").await, Some(60.0));
        assert_eq!(repo.target_of("ETH").await, Some(40.0));
    }

    #[tokio::test]
    async fn fetch_position_sources_keeps_the_latest_row_of_each_source() {
        let repo = test_repo().await;
        repo.append_ledger_entries(
            PORTFOLIO,
            &[
                entry("BTC", 1.0, Some("Ledger")),
                entry("BTC", 2.0, Some("Binance")),
                entry("BTC", 3.0, None),
            ],
        )
        .await
        .unwrap();
        repo.append_ledger_entries(PORTFOLIO, &[entry("BTC", 1.5, Some("Ledger"))])
            .await
            .unwrap();

        let sources = repo.fetch_position_sources(PORTFOLIO).await.unwrap();

        let quantities: Vec<_> = sources
            .iter()
            .map(|s| (s.notes.as_deref(), s.current_quantity))
            .collect();
        assert_eq!(
            quantities,
            [(None, 3.0), (Some("Binance"), 2.0), (Some("Ledger"), 1.5)]
        );
    }

    // --- BARCA targets -----------------------------------------------------

    #[tokio::test]
    async fn replace_barca_targets_replaces_only_that_market() {
        let repo = test_repo().await;
        repo.replace_barca_targets(
            PORTFOLIO,
            "BullMarket",
            &[barca("Base", 60.0), barca("Alt", 40.0)],
            None,
        )
        .await
        .unwrap();
        repo.replace_barca_targets(PORTFOLIO, "BearMarket", &[barca("Base", 100.0)], None)
            .await
            .unwrap();
        repo.replace_barca_targets(PORTFOLIO, "BullMarket", &[barca("Base", 100.0)], None)
            .await
            .unwrap();

        let bull = repo
            .fetch_barca_targets(PORTFOLIO, "BullMarket")
            .await
            .unwrap();
        assert_eq!(bull.len(), 1, "left-out barcas are deleted");
        assert_eq!(bull[0].target_percent, 100.0);
        assert_eq!(
            repo.fetch_barca_targets(PORTFOLIO, "BearMarket")
                .await
                .unwrap()
                .len(),
            1
        );
    }

    // --- snapshots ---------------------------------------------------------

    #[tokio::test]
    async fn record_snapshot_writes_every_history_level() {
        let repo = test_repo().await;
        repo.replace_targets(
            PORTFOLIO,
            &[target("BTC", 100.0)],
            &[entry("BTC", 1.0, None)],
            None,
        )
        .await
        .unwrap();
        let report = AllocationReport {
            total_value: 10.0,
            per_asset: vec![AssetAllocation {
                symbol: "BTC".into(),
                group: "Core".into(),
                barca: "Base".into(),
                price: 10.0,
                current_quantity: 1.0,
                value: 10.0,
                target_percent: 100.0,
                current_percent: 100.0,
                deviation: 0.0,
            }],
            ..Default::default()
        };

        repo.record_snapshot(
            PORTFOLIO,
            &AllocationSnapshot {
                timestamp: "2026-01-01T00:00:00Z".into(),
                report,
                asset_notes: HashMap::from([("BTC".to_string(), r#"{"note":1}"#.to_string())]),
            },
        )
        .await
        .unwrap();

        let extra: Option<String> =
            sqlx::query_scalar("SELECT extra FROM history_assets WHERE symbol = 'BTC'")
                .fetch_one(&repo.pool)
                .await
                .unwrap();
        assert_eq!(extra.as_deref(), Some(r#"{"note":1}"#));

        let totals = repo.fetch_total_history(PORTFOLIO).await.unwrap();
        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].total_value, Some(10.0));
        let assets = repo.fetch_asset_history(PORTFOLIO).await.unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].symbol, "BTC");
    }

    #[tokio::test]
    async fn record_snapshot_keeps_a_symbol_held_in_two_groups() {
        // Regression: history_assets was UNIQUE(timestamp, symbol), so BTC in
        // Holding was silently dropped because BTC in DeFi came first.
        let repo = test_repo().await;
        let btc = |group: &str, value: f64| AssetAllocation {
            symbol: "BTC".into(),
            group: group.into(),
            barca: "Base".into(),
            price: 10.0,
            current_quantity: value / 10.0,
            value,
            target_percent: 0.0,
            current_percent: 0.0,
            deviation: 0.0,
        };
        repo.record_snapshot(
            PORTFOLIO,
            &AllocationSnapshot {
                timestamp: "2026-01-01T00:00:00Z".into(),
                report: AllocationReport {
                    total_value: 30.0,
                    per_asset: vec![btc("DeFi", 10.0), btc("Holding", 20.0)],
                    ..Default::default()
                },
                asset_notes: HashMap::new(),
            },
        )
        .await
        .unwrap();

        let assets = repo.fetch_asset_history(PORTFOLIO).await.unwrap();
        let rows: Vec<_> = assets
            .iter()
            .map(|a| (a.group_name.as_deref(), a.value))
            .collect();
        assert_eq!(
            rows,
            [(Some("DeFi"), Some(10.0)), (Some("Holding"), Some(20.0))]
        );
        assert_eq!(
            repo.fetch_latest_prices(PORTFOLIO).await.unwrap(),
            HashMap::from([("BTC".to_string(), 10.0)])
        );
    }

    #[tokio::test]
    async fn fetch_latest_prices_reads_only_the_most_recent_snapshot() {
        let repo = test_repo().await;
        assert!(
            repo.fetch_latest_prices(PORTFOLIO)
                .await
                .unwrap()
                .is_empty()
        );

        let asset = |symbol: &str, price: f64| AssetAllocation {
            symbol: symbol.into(),
            group: "Core".into(),
            barca: "Base".into(),
            price,
            current_quantity: 1.0,
            value: price,
            target_percent: 0.0,
            current_percent: 0.0,
            deviation: 0.0,
        };
        for (timestamp, assets) in [
            (
                "2026-01-01T00:00:00+00:00",
                vec![asset("BTC", 10.0), asset("OLD", 1.0)],
            ),
            ("2026-01-02T00:00:00+00:00", vec![asset("BTC", 20.0)]),
        ] {
            repo.record_snapshot(
                PORTFOLIO,
                &AllocationSnapshot {
                    timestamp: timestamp.into(),
                    report: AllocationReport {
                        per_asset: assets,
                        ..Default::default()
                    },
                    asset_notes: HashMap::new(),
                },
            )
            .await
            .unwrap();
        }

        let prices = repo.fetch_latest_prices(PORTFOLIO).await.unwrap();
        assert_eq!(prices, HashMap::from([("BTC".to_string(), 20.0)]));
    }

    // --- portfolios ----------------------------------------------------------

    /// A second portfolio, owned by a fresh user.
    async fn second_portfolio(repo: &SqliteRepo) -> PortfolioId {
        let owner = repo
            .create_user(new_user("owner", "owner@example.com"))
            .await
            .unwrap();
        repo.create_portfolio("Spouse", owner.id.unwrap(), None)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn list_all_portfolio_ids_ignores_membership() {
        let repo = test_repo().await;
        let other = second_portfolio(&repo).await;
        // Nobody is a member of portfolio 1 here; it is still listed.
        assert_eq!(
            repo.list_all_portfolio_ids().await.unwrap(),
            [PORTFOLIO, other]
        );
    }

    #[tokio::test]
    async fn every_read_only_sees_its_own_portfolio() {
        let repo = test_repo().await;
        let other = second_portfolio(&repo).await;

        // Same symbol, group, BARCA and source in both: only portfolio_id
        // tells them apart.
        repo.import_positions(
            PORTFOLIO,
            &[target("BTC", 100.0)],
            &[entry("BTC", 1.0, Some("Ledger"))],
        )
        .await
        .unwrap();
        repo.import_positions(
            other,
            &[target("BTC", 40.0)],
            &[entry("BTC", 5.0, Some("Ledger"))],
        )
        .await
        .unwrap();
        repo.replace_barca_targets(other, "BullMarket", &[barca("Base", 100.0)], None)
            .await
            .unwrap();
        let report = AllocationReport {
            total_value: 50.0,
            per_asset: vec![AssetAllocation {
                symbol: "BTC".into(),
                group: "Core".into(),
                barca: "Base".into(),
                price: 10.0,
                current_quantity: 5.0,
                value: 50.0,
                target_percent: 40.0,
                current_percent: 100.0,
                deviation: 60.0,
            }],
            ..Default::default()
        };
        // Same timestamp in both, as two clicks in the same instant would be.
        for portfolio in [PORTFOLIO, other] {
            repo.record_snapshot(
                portfolio,
                &AllocationSnapshot {
                    timestamp: "2026-01-01T00:00:00Z".into(),
                    report: report.clone(),
                    asset_notes: HashMap::new(),
                },
            )
            .await
            .unwrap();
        }

        let mine = repo.fetch_positions(PORTFOLIO).await.unwrap();
        assert_eq!(
            (mine.len(), mine[0].current_quantity, mine[0].target_percent),
            (1, 1.0, 100.0)
        );
        let theirs = repo.fetch_positions(other).await.unwrap();
        assert_eq!(
            (
                theirs.len(),
                theirs[0].current_quantity,
                theirs[0].target_percent
            ),
            (1, 5.0, 40.0)
        );
        assert_eq!(
            repo.fetch_position_sources(PORTFOLIO).await.unwrap().len(),
            1
        );
        assert!(
            repo.fetch_barca_targets(PORTFOLIO, "BullMarket")
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(repo.fetch_total_history(PORTFOLIO).await.unwrap().len(), 1);
        assert_eq!(repo.fetch_asset_history(other).await.unwrap().len(), 1);

        // Replacing one portfolio's targets doesn't delete the other's.
        repo.replace_targets(other, &[], &[], None).await.unwrap();
        assert_eq!(
            repo.fetch_positions(PORTFOLIO).await.unwrap()[0].target_percent,
            100.0
        );
    }

    #[tokio::test]
    async fn an_insert_without_a_portfolio_fails_instead_of_landing_in_one() {
        // The safety net under every repo method: portfolio_id has no default.
        let repo = test_repo().await;
        let result = sqlx::query(
            "INSERT INTO wallet_allocations (symbol, asset_class, current_quantity) \
             VALUES ('BTC', 'crypto', 1.0)",
        )
        .execute(&repo.pool)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn membership_decides_access_and_deleting_a_user_removes_it() {
        let repo = test_repo().await;
        let other = second_portfolio(&repo).await;
        let viewer = repo
            .create_user(new_user("viewer", "viewer@example.com"))
            .await
            .unwrap()
            .id
            .unwrap();

        assert_eq!(repo.find_access(other, viewer).await.unwrap(), None);
        repo.set_member(other, viewer, PortfolioAccess::View)
            .await
            .unwrap();
        repo.set_member(other, viewer, PortfolioAccess::Manage)
            .await
            .unwrap();
        assert_eq!(
            repo.find_access(other, viewer).await.unwrap(),
            Some(PortfolioAccess::Manage),
            "set_member updates, never duplicates"
        );

        repo.delete_user(viewer).await.unwrap();
        assert_eq!(repo.find_access(other, viewer).await.unwrap(), None);
    }

    #[tokio::test]
    async fn migration_0013_moves_existing_data_into_portfolio_1() {
        // Run every migration up to 0012, write data in the old shape (no
        // portfolio_id), then run 0013 and check nothing was lost or moved.
        use sqlx::Executor;
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let migrator = sqlx::migrate!("./migrations");
        let (before, migration_13): (Vec<_>, Vec<_>) =
            migrator.iter().partition(|m| m.version < 13);
        for migration in before {
            pool.execute(&*migration.sql).await.unwrap();
        }
        pool.execute(
            "INSERT INTO users (username, password_hash, role, email) VALUES \
                ('boss', 'h', 'manager', 'boss@example.com'), \
                ('reader', 'h', 'user', 'reader@example.com'); \
             INSERT INTO wallet_allocations (symbol, group_name, barca, current_quantity, notes) \
                VALUES ('BTC', 'Core', 'Base', 1.0, 'Ledger'), ('BTC', 'Core', 'Base', 0.5, 'Binance'); \
             INSERT INTO portfolio_targets (symbol, group_name, barca, asset_class, target_percent) \
                VALUES ('BTC', 'Core', 'Base', 'crypto', 100); \
             INSERT INTO barca_targets (market, barca, target_percent) VALUES ('BullMarket', 'Base', 100); \
             INSERT INTO history_totals (timestamp, total_value) VALUES ('2026-01-01T00:00:00Z', 15);",
        )
        .await
        .unwrap();

        pool.execute(&*migration_13[0].sql).await.unwrap();

        let repo = SqliteRepo::new(pool);
        let positions = repo.fetch_positions(PortfolioId::FIRST).await.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].current_quantity, 1.5);
        assert_eq!(positions[0].target_percent, 100.0);
        assert_eq!(positions[0].source_count, 2);
        assert_eq!(
            repo.fetch_barca_targets(PortfolioId::FIRST, "BullMarket")
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repo.fetch_total_history(PortfolioId::FIRST)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repo.find_access(PortfolioId::FIRST, 1).await.unwrap(),
            Some(PortfolioAccess::Manage)
        );
        assert_eq!(
            repo.find_access(PortfolioId::FIRST, 2).await.unwrap(),
            Some(PortfolioAccess::View)
        );
    }

    // --- users -------------------------------------------------------------

    #[tokio::test]
    async fn create_user_reports_a_duplicate_username_or_email_as_unique_violation() {
        let repo = test_repo().await;
        let created = repo
            .create_user(new_user("alice", "alice@example.com"))
            .await
            .unwrap();
        assert!(created.id.is_some());

        for duplicate in [
            new_user("alice", "other@example.com"),
            new_user("bob", "alice@example.com"),
        ] {
            let err = repo.create_user(duplicate).await.unwrap_err();
            assert!(err.downcast_ref::<UniqueViolation>().is_some());
        }
    }

    #[tokio::test]
    async fn update_user_changes_only_the_given_fields_and_reports_missing_ids() {
        let repo = test_repo().await;
        let id = repo
            .create_user(new_user("bob", "bob@example.com"))
            .await
            .unwrap()
            .id
            .unwrap();

        let found = repo
            .update_user(
                id,
                UserUpdate {
                    role: Some("manager"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(found);
        let user = repo.find_user_by_id(id).await.unwrap().unwrap();
        assert_eq!(user.role, "manager");
        assert_eq!(user.email, "bob@example.com");

        assert!(!repo.update_user(9999, UserUpdate::default()).await.unwrap());
    }

    #[tokio::test]
    async fn delete_user_works_even_after_they_edited_targets() {
        // Regression: foreign keys are enforced, and portfolio_targets /
        // barca_targets.updated_by reference users(id), so deleting a
        // manager who had ever saved targets used to fail.
        let repo = test_repo().await;
        let id = repo
            .create_user(new_user("carol", "carol@example.com"))
            .await
            .unwrap()
            .id
            .unwrap();
        repo.store_refresh_token(id, "token-hash", "2999-01-01T00:00:00Z")
            .await
            .unwrap();
        repo.replace_targets(PORTFOLIO, &[target("BTC", 100.0)], &[], Some(id))
            .await
            .unwrap();
        repo.replace_barca_targets(PORTFOLIO, "BullMarket", &[barca("Base", 100.0)], Some(id))
            .await
            .unwrap();
        // trades.recorded_by references users(id) too.
        repo.record_trade(
            PORTFOLIO,
            &trade_record("BTC", Some(id)),
            &[SourceChange {
                entry: entry("BTC", 1.0, Some("Binance")),
                before: 0.0,
            }],
        )
        .await
        .unwrap();

        assert!(repo.delete_user(id).await.unwrap());

        assert!(repo.find_user_by_id(id).await.unwrap().is_none());
        assert!(
            repo.find_refresh_token("token-hash")
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(repo.target_of("BTC").await, Some(100.0), "targets are kept");
        let recorded_by: Option<i64> = sqlx::query_scalar("SELECT recorded_by FROM trades")
            .fetch_one(&repo.pool)
            .await
            .unwrap();
        assert_eq!(recorded_by, None, "the trade is kept, without its author");
        assert!(
            !repo.delete_user(id).await.unwrap(),
            "second delete finds nothing"
        );
    }

    fn trade_record(symbol: &str, recorded_by: Option<i64>) -> TradeRecord {
        TradeRecord {
            side: crate::domain::models::TradeSide::Buy,
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            asset_class: "crypto".to_string(),
            source: Some("Binance".to_string()),
            quantity: 1.0,
            price: 1.0,
            cash_symbol: None,
            cash_source: None,
            cash_quantity: None,
            recorded_by,
        }
    }

    #[tokio::test]
    async fn a_change_worked_out_from_a_stale_quantity_writes_nothing() {
        let repo = test_repo().await;
        repo.append_ledger_entries(PORTFOLIO, &[entry("BTC", 0.5, Some("Binance"))])
            .await
            .unwrap();
        let change = |before| SourceChange {
            entry: entry("BTC", 0.7, Some("Binance")),
            before,
        };

        // Worked out from 0.4, but the source holds 0.5 now.
        let err = repo
            .record_trade(PORTFOLIO, &trade_record("BTC", None), &[change(0.4)])
            .await
            .unwrap_err();
        assert!(err.downcast_ref::<StaleSource>().is_some(), "{err}");
        let trades: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM trades")
            .fetch_one(&repo.pool)
            .await
            .unwrap();
        assert_eq!(trades, 0, "the log row is rolled back too");

        // From the right quantity it goes through.
        repo.record_trade(PORTFOLIO, &trade_record("BTC", None), &[change(0.5)])
            .await
            .unwrap();
        let sources = repo.fetch_position_sources(PORTFOLIO).await.unwrap();
        assert_eq!(sources[0].current_quantity, 0.7);
    }

    #[tokio::test]
    async fn revoke_refresh_token_marks_it_without_deleting_it() {
        let repo = test_repo().await;
        let id = repo
            .create_user(new_user("dave", "dave@example.com"))
            .await
            .unwrap()
            .id
            .unwrap();
        repo.store_refresh_token(id, "token-hash", "2999-01-01T00:00:00Z")
            .await
            .unwrap();

        repo.revoke_refresh_token("token-hash").await.unwrap();

        let token = repo
            .find_refresh_token("token-hash")
            .await
            .unwrap()
            .unwrap();
        assert!(token.revoked_at.is_some());
    }
}
