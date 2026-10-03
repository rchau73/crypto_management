//! SQLite implementation of every repository port in `domain::repository`.

use crate::domain::models::{
    AllocationSnapshot, AssetHistoryRow, BarcaHistoryRow, BarcaTarget, GroupHistoryRow,
    LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PositionSource, RefreshToken, TotalHistoryRow,
    User, WalletPosition,
};
use crate::domain::repository::{
    BarcaTargetRepo, NewUser, PortfolioRepo, RefreshTokenRepo, RepoError, RepoResult, SnapshotRepo,
    UniqueViolation, UserRepo, UserUpdate,
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
    entries: &[LedgerEntry],
) -> RepoResult<()> {
    for entry in entries {
        sqlx::query(
            "INSERT INTO wallet_allocations \
             (symbol, group_name, barca, asset_class, current_quantity, last_price, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
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

#[async_trait]
impl PortfolioRepo for SqliteRepo {
    async fn fetch_positions(&self) -> RepoResult<Vec<WalletPosition>> {
        let rows = sqlx::query_as::<_, WalletPosition>(
            "SELECT symbol, group_name, barca, asset_class, target_percent, current_quantity, \
                    last_price, notes, source_count \
             FROM wallet_allocations_current \
             ORDER BY symbol, group_name, barca, asset_class",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_position_sources(&self) -> RepoResult<Vec<PositionSource>> {
        // Same "latest row per source" rule as the wallet_allocations_current
        // view (migration 0011), without the SUM.
        let rows = sqlx::query_as::<_, PositionSource>(
            "SELECT symbol, group_name, barca, asset_class, \
                    COALESCE(current_quantity, 0.0) AS current_quantity, notes \
             FROM ( \
                 SELECT *, ROW_NUMBER() OVER ( \
                     PARTITION BY symbol, group_name, barca, asset_class, COALESCE(notes, '') \
                     ORDER BY created_at DESC, id DESC \
                 ) AS rn \
                 FROM wallet_allocations \
             ) \
             WHERE rn = 1 \
             ORDER BY symbol, group_name, barca, asset_class, notes",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn append_ledger_entries(&self, entries: &[LedgerEntry]) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        insert_ledger_entries(&mut tx, entries).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn replace_targets(
        &self,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
        updated_by: Option<i64>,
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM portfolio_targets")
            .execute(&mut *tx)
            .await?;
        for target in targets {
            sqlx::query(
                "INSERT INTO portfolio_targets \
                 (symbol, group_name, barca, asset_class, target_percent, updated_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
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
        insert_ledger_entries(&mut tx, new_entries).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn import_positions(
        &self,
        targets: &[NewPortfolioTarget],
        new_entries: &[LedgerEntry],
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        for target in targets {
            sqlx::query(
                "INSERT OR IGNORE INTO portfolio_targets \
                 (symbol, group_name, barca, asset_class, target_percent) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(&target.key.symbol)
            .bind(&target.key.group_name)
            .bind(&target.key.barca)
            .bind(&target.key.asset_class)
            .bind(target.target_percent)
            .execute(&mut *tx)
            .await?;
        }
        insert_ledger_entries(&mut tx, new_entries).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl BarcaTargetRepo for SqliteRepo {
    async fn fetch_barca_targets(&self, market: &str) -> RepoResult<Vec<BarcaTarget>> {
        let rows = sqlx::query_as::<_, BarcaTarget>(
            "SELECT * FROM barca_targets WHERE market = ?1 ORDER BY barca ASC",
        )
        .bind(market)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn replace_barca_targets(
        &self,
        market: &str,
        targets: &[NewBarcaTarget],
        updated_by: Option<i64>,
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM barca_targets WHERE market = ?1")
            .bind(market)
            .execute(&mut *tx)
            .await?;
        for target in targets {
            sqlx::query(
                "INSERT INTO barca_targets (market, barca, target_percent, updated_by) \
                 VALUES (?1, ?2, ?3, ?4)",
            )
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
    async fn record_snapshot(&self, snapshot: &AllocationSnapshot) -> RepoResult<()> {
        let ts = &snapshot.timestamp;
        let report = &snapshot.report;
        let mut tx = self.pool.begin().await?;

        sqlx::query("INSERT INTO allocations (computed_at, payload) VALUES (?1, ?2)")
            .bind(ts)
            .bind(serde_json::to_string(report)?)
            .execute(&mut *tx)
            .await?;

        // `INSERT OR IGNORE`: history_assets is UNIQUE(timestamp, symbol),
        // so a symbol held in two groups keeps only its first row per
        // snapshot (a known limitation of that table's original schema).
        for a in &report.per_asset {
            sqlx::query(
                "INSERT OR IGNORE INTO history_assets \
                 (timestamp, symbol, group_name, barca, price, current_quantity, value, \
                  target_percent, current_percent, extra) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
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
                 (timestamp, group_name, value, current_percent, target_percent) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
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
                 (timestamp, barca, value, current_percent, target_percent) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(ts)
            .bind(&b.barca)
            .bind(b.value)
            .bind(b.current_percent)
            .bind(b.target_percent)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT OR REPLACE INTO history_totals (timestamp, total_value) VALUES (?1, ?2)",
        )
        .bind(ts)
        .bind(report.total_value)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn fetch_asset_history(&self) -> RepoResult<Vec<AssetHistoryRow>> {
        let rows = sqlx::query_as(
            "SELECT timestamp, symbol, group_name, barca, price, current_quantity, value, \
                    target_percent, current_percent, deviation_percent, value_deviation \
             FROM asset_variance_history ORDER BY timestamp, symbol",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_group_history(&self) -> RepoResult<Vec<GroupHistoryRow>> {
        let rows = sqlx::query_as(
            "SELECT timestamp, group_name, value, current_percent, target_percent, deviation_percent \
             FROM group_variance_history ORDER BY timestamp, group_name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_barca_history(&self) -> RepoResult<Vec<BarcaHistoryRow>> {
        let rows = sqlx::query_as(
            "SELECT timestamp, barca, value, current_percent, target_percent, deviation_percent \
             FROM barca_variance_history ORDER BY timestamp, barca",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn fetch_total_history(&self) -> RepoResult<Vec<TotalHistoryRow>> {
        let rows =
            sqlx::query_as("SELECT timestamp, total_value FROM history_totals ORDER BY timestamp")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn fetch_latest_prices(&self) -> RepoResult<HashMap<String, f64>> {
        // Timestamps are RFC 3339 in UTC, so MAX() is the most recent one.
        let rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT symbol, price FROM history_assets \
             WHERE timestamp = (SELECT MAX(timestamp) FROM history_totals) \
               AND price IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().collect())
    }
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
        sqlx::query("UPDATE portfolio_targets SET updated_by = NULL WHERE updated_by = ?1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE barca_targets SET updated_by = NULL WHERE updated_by = ?1")
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
        repo.append_ledger_entries(&[
            entry("BTC", 1.0, Some("Ledger")),
            entry("BTC", 0.5, Some("Binance")),
            entry("BTC", 2.0, Some("Ledger")), // supersedes Ledger's 1.0
        ])
        .await
        .unwrap();
        repo.replace_targets(&[target("BTC", 40.0)], &[], None)
            .await
            .unwrap();

        let positions = repo.fetch_positions().await.unwrap();
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
        repo.append_ledger_entries(&[entry("BTC", 1.0, None), entry("BTC", 2.0, Some("Binance"))])
            .await
            .unwrap();

        let position = &repo.fetch_positions().await.unwrap()[0];
        assert_eq!(position.notes.as_deref(), Some("Binance"));
        assert_eq!(position.source_count, 2);
    }

    #[tokio::test]
    async fn a_position_without_a_target_defaults_to_zero() {
        let repo = test_repo().await;
        repo.append_ledger_entries(&[entry("SOL", 1.0, None)])
            .await
            .unwrap();
        assert_eq!(repo.fetch_positions().await.unwrap()[0].target_percent, 0.0);
    }

    #[tokio::test]
    async fn the_same_symbol_in_two_asset_classes_stays_two_positions() {
        let repo = test_repo().await;
        let mut us = entry("IVV", 5.0, None);
        us.asset_class = "us-indices".to_string();
        repo.append_ledger_entries(&[us, entry("IVV", 1.0, None)])
            .await
            .unwrap();
        assert_eq!(repo.fetch_positions().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn replace_targets_replaces_the_whole_set_and_appends_new_entries() {
        let repo = test_repo().await;
        repo.replace_targets(&[target("BTC", 60.0), target("ETH", 40.0)], &[], None)
            .await
            .unwrap();
        repo.replace_targets(&[target("BTC", 100.0)], &[entry("BTC", 1.0, None)], None)
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
        repo.replace_targets(&[target("BTC", 100.0)], &[], None)
            .await
            .unwrap();

        // A duplicate key fails mid-transaction: neither the delete, nor
        // the new targets, nor the ledger entry may be kept.
        let err = repo
            .replace_targets(
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
        repo.replace_targets(&[target("BTC", 60.0)], &[], None)
            .await
            .unwrap();

        repo.import_positions(&[target("BTC", 99.0), target("ETH", 40.0)], &[])
            .await
            .unwrap();

        assert_eq!(repo.target_of("BTC").await, Some(60.0));
        assert_eq!(repo.target_of("ETH").await, Some(40.0));
    }

    #[tokio::test]
    async fn fetch_position_sources_keeps_the_latest_row_of_each_source() {
        let repo = test_repo().await;
        repo.append_ledger_entries(&[
            entry("BTC", 1.0, Some("Ledger")),
            entry("BTC", 2.0, Some("Binance")),
            entry("BTC", 3.0, None),
        ])
        .await
        .unwrap();
        repo.append_ledger_entries(&[entry("BTC", 1.5, Some("Ledger"))])
            .await
            .unwrap();

        let sources = repo.fetch_position_sources().await.unwrap();

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
            "BullMarket",
            &[barca("Base", 60.0), barca("Alt", 40.0)],
            None,
        )
        .await
        .unwrap();
        repo.replace_barca_targets("BearMarket", &[barca("Base", 100.0)], None)
            .await
            .unwrap();
        repo.replace_barca_targets("BullMarket", &[barca("Base", 100.0)], None)
            .await
            .unwrap();

        let bull = repo.fetch_barca_targets("BullMarket").await.unwrap();
        assert_eq!(bull.len(), 1, "left-out barcas are deleted");
        assert_eq!(bull[0].target_percent, 100.0);
        assert_eq!(
            repo.fetch_barca_targets("BearMarket").await.unwrap().len(),
            1
        );
    }

    // --- snapshots ---------------------------------------------------------

    #[tokio::test]
    async fn record_snapshot_writes_every_history_level() {
        let repo = test_repo().await;
        repo.replace_targets(&[target("BTC", 100.0)], &[entry("BTC", 1.0, None)], None)
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

        repo.record_snapshot(&AllocationSnapshot {
            timestamp: "2026-01-01T00:00:00Z".into(),
            report,
            asset_notes: HashMap::from([("BTC".to_string(), r#"{"note":1}"#.to_string())]),
        })
        .await
        .unwrap();

        let extra: Option<String> =
            sqlx::query_scalar("SELECT extra FROM history_assets WHERE symbol = 'BTC'")
                .fetch_one(&repo.pool)
                .await
                .unwrap();
        assert_eq!(extra.as_deref(), Some(r#"{"note":1}"#));

        let totals = repo.fetch_total_history().await.unwrap();
        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].total_value, Some(10.0));
        let assets = repo.fetch_asset_history().await.unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].symbol, "BTC");
    }

    #[tokio::test]
    async fn fetch_latest_prices_reads_only_the_most_recent_snapshot() {
        let repo = test_repo().await;
        assert!(repo.fetch_latest_prices().await.unwrap().is_empty());

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
            repo.record_snapshot(&AllocationSnapshot {
                timestamp: timestamp.into(),
                report: AllocationReport {
                    per_asset: assets,
                    ..Default::default()
                },
                asset_notes: HashMap::new(),
            })
            .await
            .unwrap();
        }

        let prices = repo.fetch_latest_prices().await.unwrap();
        assert_eq!(prices, HashMap::from([("BTC".to_string(), 20.0)]));
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
        repo.replace_targets(&[target("BTC", 100.0)], &[], Some(id))
            .await
            .unwrap();
        repo.replace_barca_targets("BullMarket", &[barca("Base", 100.0)], Some(id))
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
        assert!(
            !repo.delete_user(id).await.unwrap(),
            "second delete finds nothing"
        );
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
