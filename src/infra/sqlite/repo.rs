use crate::domain::models::{
    AllocationRecord, AssetHistoryRow, AssetSnapshot, BarcaHistoryRow, BarcaSnapshot, BarcaTarget,
    GroupHistoryRow, GroupSnapshot, PortfolioTarget, RefreshToken, TotalSnapshot, User,
    WalletAllocation,
};
use crate::domain::repository::{
    BarcaTargetInput, BarcaTargetRepo, HistoryRepo, NewUser, PortfolioTargetInput,
    PortfolioTargetRepo, RefreshTokenRepo, RepoResult, UserRepo, UserUpdate,
};
use async_trait::async_trait;
use sqlx::{QueryBuilder, SqlitePool};

pub struct SqliteRepo {
    pub pool: SqlitePool,
}

impl SqliteRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HistoryRepo for SqliteRepo {
    async fn insert_asset_snapshot(&self, snap: &AssetSnapshot) -> RepoResult<()> {
        let extra = snap.extra.as_ref().map(|v| v.to_string());
        sqlx::query(
            r#"INSERT OR IGNORE INTO history_assets (timestamp, symbol, group_name, barca, price, current_quantity, value, target_percent, current_percent, market_cap, fdv, volume_24h, percent_change_24h, percent_change_7d, extra)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
        )
        .bind(&snap.timestamp)
        .bind(&snap.symbol)
        .bind(&snap.group_name)
        .bind(&snap.barca)
        .bind(snap.price)
        .bind(snap.current_quantity)
        .bind(snap.value)
        .bind(snap.target_percent)
        .bind(snap.current_percent)
        .bind(snap.market_cap)
        .bind(snap.fdv)
        .bind(snap.volume_24h)
        .bind(snap.percent_change_24h)
        .bind(snap.percent_change_7d)
        .bind(extra)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_barca_snapshot(&self, snap: &BarcaSnapshot) -> RepoResult<()> {
        let extra = snap.extra.as_ref().map(|v| v.to_string());
        sqlx::query("INSERT OR IGNORE INTO history_barca (timestamp, barca, value, current_percent, target_percent, extra) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind(&snap.timestamp)
            .bind(&snap.barca)
            .bind(snap.value)
            .bind(snap.current_percent)
            .bind(snap.target_percent)
            .bind(extra)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn insert_group_snapshot(&self, snap: &GroupSnapshot) -> RepoResult<()> {
        let extra = snap.extra.as_ref().map(|v| v.to_string());
        sqlx::query("INSERT OR IGNORE INTO history_groups (timestamp, group_name, value, current_percent, target_percent, extra) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind(&snap.timestamp)
            .bind(&snap.group_name)
            .bind(snap.value)
            .bind(snap.current_percent)
            .bind(snap.target_percent)
            .bind(extra)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn insert_total_snapshot(&self, snap: &TotalSnapshot) -> RepoResult<()> {
        let extra = snap.extra.as_ref().map(|v| v.to_string());
        sqlx::query("INSERT OR REPLACE INTO history_totals (timestamp, total_value, extra) VALUES (?1, ?2, ?3)")
            .bind(&snap.timestamp)
            .bind(snap.total_value)
            .bind(extra)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn fetch_assets(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<AssetHistoryRow>> {
        let mut qb = QueryBuilder::new(
            "SELECT timestamp, symbol, group_name, barca, price, current_quantity, value, target_percent, current_percent, deviation_percent, value_deviation FROM asset_variance_history",
        );
        if from.is_some() || to.is_some() {
            qb.push(" WHERE ");
            let mut first = true;
            if let Some(f) = from {
                qb.push("timestamp >= ");
                qb.push_bind(f);
                first = false;
            }
            if let Some(t) = to {
                if !first {
                    qb.push(" AND ");
                }
                qb.push("timestamp <= ");
                qb.push_bind(t);
            }
        }
        qb.push(" ORDER BY timestamp ASC, symbol ASC");
        let rows = qb
            .build_query_as::<AssetHistoryRow>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn fetch_barca(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<BarcaHistoryRow>> {
        let mut qb = QueryBuilder::new(
            "SELECT timestamp, barca, value, current_percent, target_percent, deviation_percent FROM barca_variance_history",
        );
        if from.is_some() || to.is_some() {
            qb.push(" WHERE ");
            let mut first = true;
            if let Some(f) = from {
                qb.push("timestamp >= ");
                qb.push_bind(f);
                first = false;
            }
            if let Some(t) = to {
                if !first {
                    qb.push(" AND ");
                }
                qb.push("timestamp <= ");
                qb.push_bind(t);
            }
        }
        qb.push(" ORDER BY timestamp ASC, barca ASC");
        let rows = qb
            .build_query_as::<BarcaHistoryRow>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn fetch_groups(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<GroupHistoryRow>> {
        let mut qb = QueryBuilder::new(
            "SELECT timestamp, group_name, value, current_percent, target_percent, deviation_percent FROM group_variance_history",
        );
        if from.is_some() || to.is_some() {
            qb.push(" WHERE ");
            let mut first = true;
            if let Some(f) = from {
                qb.push("timestamp >= ");
                qb.push_bind(f);
                first = false;
            }
            if let Some(t) = to {
                if !first {
                    qb.push(" AND ");
                }
                qb.push("timestamp <= ");
                qb.push_bind(t);
            }
        }
        qb.push(" ORDER BY timestamp ASC, group_name ASC");
        let rows = qb
            .build_query_as::<GroupHistoryRow>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn fetch_totals(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RepoResult<Vec<TotalSnapshot>> {
        let mut q = String::from("SELECT * FROM history_totals");
        if from.is_some() || to.is_some() {
            q.push_str(" WHERE ");
            let mut clauses = Vec::new();
            if from.is_some() {
                clauses.push("timestamp >= ?1");
            }
            if to.is_some() {
                clauses.push("timestamp <= ?2");
            }
            q.push_str(&clauses.join(" AND "));
        }
        let rows = sqlx::query_as::<_, TotalSnapshot>(&q)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    // wallet allocations ledger
    async fn insert_wallet_allocation(&self, wa: &WalletAllocation) -> RepoResult<()> {
        sqlx::query("INSERT INTO wallet_allocations (symbol, group_name, barca, target_percent, current_quantity, last_price, notes, asset_class) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
            .bind(&wa.symbol)
            .bind(&wa.group_name)
            .bind(&wa.barca)
            .bind(wa.target_percent)
            .bind(wa.current_quantity)
            .bind(wa.last_price)
            .bind(wa.notes.as_deref())
            .bind(&wa.asset_class)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn bulk_insert_wallet_allocations(&self, rows: &[WalletAllocation]) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        for wa in rows {
            sqlx::query("INSERT INTO wallet_allocations (symbol, group_name, barca, target_percent, current_quantity, last_price, notes, asset_class) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
                .bind(&wa.symbol)
                .bind(&wa.group_name)
                .bind(&wa.barca)
                .bind(wa.target_percent)
                .bind(wa.current_quantity)
                .bind(wa.last_price)
                .bind(wa.notes.as_deref())
                .bind(&wa.asset_class)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn fetch_current_wallet_allocations(&self) -> RepoResult<Vec<WalletAllocation>> {
        let rows =
            sqlx::query_as::<_, WalletAllocation>("SELECT * FROM wallet_allocations_current")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn fetch_wallet_allocation_history(
        &self,
        symbol: &str,
    ) -> RepoResult<Vec<WalletAllocation>> {
        let rows = sqlx::query_as::<_, WalletAllocation>(
            "SELECT * FROM wallet_allocations WHERE symbol = ?1 ORDER BY created_at DESC",
        )
        .bind(symbol)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn persist_allocation_record(&self, rec: &AllocationRecord) -> RepoResult<()> {
        sqlx::query("INSERT INTO allocations (computed_at, payload) VALUES (?1, ?2)")
            .bind(&rec.computed_at)
            .bind(rec.payload.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
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
        let result = sqlx::query(
            "INSERT INTO users (username, password_hash, role, email, phone) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(new_user.username)
        .bind(new_user.password_hash)
        .bind(new_user.role)
        .bind(new_user.email)
        .bind(new_user.phone)
        .execute(&self.pool)
        .await?;
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
            .bind(result.last_insert_rowid())
            .fetch_one(&self.pool)
            .await?;
        Ok(user)
    }

    async fn update_user(&self, id: i64, update: UserUpdate<'_>) -> RepoResult<()> {
        sqlx::query(
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
        .await?;
        Ok(())
    }

    async fn delete_user(&self, id: i64) -> RepoResult<()> {
        // Refresh tokens have no ON DELETE CASCADE, so clear them explicitly.
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM users WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
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
        sqlx::query("UPDATE refresh_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE token_hash = ?1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
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
        targets: &[BarcaTargetInput<'_>],
        updated_by: Option<i64>,
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM barca_targets WHERE market = ?1")
            .bind(market)
            .execute(&mut *tx)
            .await?;
        for target in targets {
            sqlx::query(
                "INSERT INTO barca_targets (market, barca, target_percent, updated_by) VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(market)
            .bind(target.barca)
            .bind(target.target_percent)
            .bind(updated_by)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl PortfolioTargetRepo for SqliteRepo {
    async fn fetch_portfolio_targets(&self) -> RepoResult<Vec<PortfolioTarget>> {
        let rows = sqlx::query_as::<_, PortfolioTarget>(
            "SELECT * FROM portfolio_targets ORDER BY symbol ASC, group_name ASC, barca ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn replace_portfolio_targets(
        &self,
        targets: &[PortfolioTargetInput<'_>],
        updated_by: Option<i64>,
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM portfolio_targets")
            .execute(&mut *tx)
            .await?;
        for target in targets {
            sqlx::query(
                "INSERT INTO portfolio_targets (symbol, group_name, barca, asset_class, target_percent, updated_by) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(target.symbol)
            .bind(target.group_name)
            .bind(target.barca)
            .bind(target.asset_class)
            .bind(target.target_percent)
            .bind(updated_by)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn seed_portfolio_targets_if_absent(
        &self,
        targets: &[PortfolioTargetInput<'_>],
    ) -> RepoResult<()> {
        let mut tx = self.pool.begin().await?;
        for target in targets {
            sqlx::query(
                "INSERT OR IGNORE INTO portfolio_targets (symbol, group_name, barca, asset_class, target_percent) VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(target.symbol)
            .bind(target.group_name)
            .bind(target.barca)
            .bind(target.asset_class)
            .bind(target.target_percent)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wallet_allocations_current_sums_quantity_across_sources_and_reads_target_from_portfolio_targets()
     {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteRepo::new(pool.clone());

        let wa1 = WalletAllocation {
            id: None,
            symbol: "BTC".to_string(),
            group_name: Some("Base".to_string()),
            barca: Some("Base".to_string()),
            target_percent: None, // vestigial — the view no longer reads this
            current_quantity: Some(1.0),
            last_price: Some(10.0),
            notes: Some("Ledger".to_string()),
            asset_class: "crypto".to_string(),
            created_at: None,
        };
        // A second ledger entry for a different source/wallet (distinct
        // notes) — its quantity must add to wa1's.
        let wa2 = WalletAllocation {
            current_quantity: Some(0.5),
            notes: Some("Binance".to_string()),
            ..wa1.clone()
        };
        repo.insert_wallet_allocation(&wa1).await.unwrap();
        repo.insert_wallet_allocation(&wa2).await.unwrap();

        repo.replace_portfolio_targets(
            &[PortfolioTargetInput {
                symbol: "BTC",
                group_name: "Base",
                barca: "Base",
                asset_class: "crypto",
                target_percent: 40.0,
            }],
            None,
        )
        .await
        .unwrap();

        let rows = repo.fetch_current_wallet_allocations().await.unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!((row.current_quantity.unwrap() - 1.5).abs() < f64::EPSILON);
        assert_eq!(row.target_percent.unwrap(), 40.0);
    }

    #[tokio::test]
    async fn wallet_allocations_current_defaults_target_to_zero_when_none_is_set_yet() {
        let repo = in_memory_repo().await;
        repo.insert_wallet_allocation(&WalletAllocation {
            id: None,
            symbol: "SOL".to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            target_percent: None,
            current_quantity: Some(1.0),
            last_price: None,
            notes: None,
            asset_class: "crypto".to_string(),
            created_at: None,
        })
        .await
        .unwrap();

        let rows = repo.fetch_current_wallet_allocations().await.unwrap();
        assert_eq!(rows[0].target_percent, Some(0.0));
    }

    #[tokio::test]
    async fn editing_the_target_of_a_multi_source_asset_via_portfolio_targets_always_applies() {
        // Regression test for a real bug: when target_percent lived on
        // wallet_allocations, only one historical source partition (e.g.
        // "Binance") carried a non-zero value; every other source partition
        // (e.g. "Ledger") had target_percent=0. No aggregation strategy over
        // that append-only, per-source-partitioned ledger was fully robust:
        // MAX() couldn't decrease, and "latest row overall" depended on
        // arbitrary CSV import ordering. Moving target_percent to its own
        // mutable portfolio_targets table (this test) sidesteps the problem
        // entirely — a target edit is a plain replace, always wins.
        let repo = in_memory_repo().await;
        let source_a = WalletAllocation {
            id: None,
            symbol: "USDT".to_string(),
            group_name: Some("Holding".to_string()),
            barca: Some("Caixa".to_string()),
            target_percent: None,
            current_quantity: Some(100.0),
            last_price: None,
            notes: Some("Binance".to_string()),
            asset_class: "crypto".to_string(),
            created_at: None,
        };
        let source_b = WalletAllocation {
            current_quantity: Some(50.0),
            notes: Some("Ledger".to_string()),
            ..source_a.clone()
        };
        repo.insert_wallet_allocation(&source_a).await.unwrap();
        repo.insert_wallet_allocation(&source_b).await.unwrap();

        let target = |target_percent: f64| PortfolioTargetInput {
            symbol: "USDT",
            group_name: "Holding",
            barca: "Caixa",
            asset_class: "crypto",
            target_percent,
        };

        repo.replace_portfolio_targets(&[target(30.0)], None)
            .await
            .unwrap();
        let before = repo.fetch_current_wallet_allocations().await.unwrap();
        assert_eq!(before[0].target_percent, Some(30.0));
        assert_eq!(before[0].current_quantity, Some(150.0));

        repo.replace_portfolio_targets(&[target(29.0)], None)
            .await
            .unwrap();
        let after = repo.fetch_current_wallet_allocations().await.unwrap();
        assert_eq!(
            after[0].target_percent,
            Some(29.0),
            "a target decrease must always apply, not lose to a stale ledger value"
        );
        assert_eq!(
            after[0].current_quantity,
            Some(150.0),
            "quantity must be unaffected by a target-only edit"
        );
    }

    fn portfolio_target(symbol: &str, target_percent: f64) -> PortfolioTargetInput<'_> {
        PortfolioTargetInput {
            symbol,
            group_name: "Core",
            barca: "Base",
            asset_class: "crypto",
            target_percent,
        }
    }

    #[tokio::test]
    async fn replace_portfolio_targets_round_trips() {
        let repo = in_memory_repo().await;
        repo.replace_portfolio_targets(
            &[portfolio_target("BTC", 60.0), portfolio_target("ETH", 40.0)],
            None,
        )
        .await
        .unwrap();

        let fetched = repo.fetch_portfolio_targets().await.unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(
            fetched
                .iter()
                .find(|t| t.symbol == "BTC")
                .unwrap()
                .target_percent,
            60.0
        );
    }

    #[tokio::test]
    async fn replace_portfolio_targets_drops_symbols_not_included_in_the_new_set() {
        let repo = in_memory_repo().await;
        repo.replace_portfolio_targets(
            &[portfolio_target("BTC", 60.0), portfolio_target("ETH", 40.0)],
            None,
        )
        .await
        .unwrap();

        repo.replace_portfolio_targets(&[portfolio_target("BTC", 100.0)], None)
            .await
            .unwrap();

        let fetched = repo.fetch_portfolio_targets().await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].symbol, "BTC");
    }

    #[tokio::test]
    async fn seed_portfolio_targets_if_absent_never_overwrites_an_existing_target() {
        let repo = in_memory_repo().await;
        repo.replace_portfolio_targets(&[portfolio_target("BTC", 60.0)], None)
            .await
            .unwrap();

        // A CSV re-import specifying a different value for BTC must not
        // clobber the manager-set target — "local always overcomes CSV".
        repo.seed_portfolio_targets_if_absent(&[
            portfolio_target("BTC", 99.0),
            portfolio_target("ETH", 40.0),
        ])
        .await
        .unwrap();

        let fetched = repo.fetch_portfolio_targets().await.unwrap();
        assert_eq!(
            fetched
                .iter()
                .find(|t| t.symbol == "BTC")
                .unwrap()
                .target_percent,
            60.0,
            "seeding must never overwrite an existing target"
        );
        assert_eq!(
            fetched
                .iter()
                .find(|t| t.symbol == "ETH")
                .unwrap()
                .target_percent,
            40.0,
            "seeding must still insert a target for a genuinely new symbol"
        );
    }

    async fn in_memory_repo() -> SqliteRepo {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        SqliteRepo::new(pool)
    }

    fn new_user<'a>(
        username: &'a str,
        password_hash: &'a str,
        role: &'a str,
        email: &'a str,
    ) -> NewUser<'a> {
        NewUser {
            username,
            password_hash,
            role,
            email,
            phone: None,
        }
    }

    #[tokio::test]
    async fn create_user_round_trips_and_never_collides_on_username() {
        let repo = in_memory_repo().await;
        let created = repo
            .create_user(new_user("alice", "hash1", "admin", "alice@example.com"))
            .await
            .unwrap();
        assert_eq!(created.username, "alice");
        assert_eq!(created.role, "admin");
        assert_eq!(created.email, "alice@example.com");
        assert_eq!(created.phone, None);

        let fetched = repo.find_user_by_username("alice").await.unwrap().unwrap();
        assert_eq!(fetched.id, created.id);

        // UNIQUE(username) must reject a duplicate rather than silently overwrite.
        let duplicate = repo
            .create_user(new_user("alice", "hash2", "user", "alice2@example.com"))
            .await;
        assert!(duplicate.is_err());
    }

    #[tokio::test]
    async fn create_user_rejects_a_duplicate_email_even_with_a_different_username() {
        let repo = in_memory_repo().await;
        repo.create_user(new_user("alice", "hash1", "user", "shared@example.com"))
            .await
            .unwrap();
        let duplicate = repo
            .create_user(new_user("alice2", "hash2", "user", "shared@example.com"))
            .await;
        assert!(duplicate.is_err());
    }

    #[tokio::test]
    async fn phone_is_optional_and_can_be_set_on_create() {
        let repo = in_memory_repo().await;
        let with_phone = NewUser {
            username: "erin",
            password_hash: "hash",
            role: "user",
            email: "erin@example.com",
            phone: Some("+1-555-0100"),
        };
        let created = repo.create_user(with_phone).await.unwrap();
        assert_eq!(created.phone.as_deref(), Some("+1-555-0100"));
    }

    #[tokio::test]
    async fn find_user_by_username_returns_none_when_absent() {
        let repo = in_memory_repo().await;
        assert!(repo.find_user_by_username("ghost").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn update_user_only_changes_the_fields_passed() {
        let repo = in_memory_repo().await;
        let user = repo
            .create_user(new_user("bob", "old-hash", "user", "bob@example.com"))
            .await
            .unwrap();

        repo.update_user(
            user.id.unwrap(),
            UserUpdate {
                role: Some("manager"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let after_role_change = repo
            .find_user_by_id(user.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after_role_change.role, "manager");
        assert_eq!(after_role_change.password_hash, "old-hash"); // untouched
        assert_eq!(after_role_change.email, "bob@example.com"); // untouched

        repo.update_user(
            user.id.unwrap(),
            UserUpdate {
                password_hash: Some("new-hash"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let after_password_change = repo
            .find_user_by_id(user.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after_password_change.role, "manager"); // untouched
        assert_eq!(after_password_change.password_hash, "new-hash");

        repo.update_user(
            user.id.unwrap(),
            UserUpdate {
                email: Some("bob2@example.com"),
                phone: Some("+1-555-0199"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let after_contact_change = repo
            .find_user_by_id(user.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after_contact_change.email, "bob2@example.com");
        assert_eq!(after_contact_change.phone.as_deref(), Some("+1-555-0199"));
        assert_eq!(after_contact_change.password_hash, "new-hash"); // untouched
    }

    #[tokio::test]
    async fn delete_user_also_clears_their_refresh_tokens() {
        let repo = in_memory_repo().await;
        let user = repo
            .create_user(new_user("carol", "hash", "user", "carol@example.com"))
            .await
            .unwrap();
        repo.store_refresh_token(user.id.unwrap(), "token-hash", "2999-01-01T00:00:00Z")
            .await
            .unwrap();

        repo.delete_user(user.id.unwrap()).await.unwrap();

        assert!(
            repo.find_user_by_id(user.id.unwrap())
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            repo.find_refresh_token("token-hash")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn revoke_refresh_token_marks_it_revoked_without_deleting_it() {
        let repo = in_memory_repo().await;
        let user = repo
            .create_user(new_user("dave", "hash", "user", "dave@example.com"))
            .await
            .unwrap();
        repo.store_refresh_token(user.id.unwrap(), "token-hash", "2999-01-01T00:00:00Z")
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

    #[tokio::test]
    async fn bulk_insert_wallet_allocations_inserts_every_row_in_one_transaction() {
        let repo = in_memory_repo().await;
        let rows = vec![
            WalletAllocation {
                id: None,
                symbol: "BTC".to_string(),
                group_name: Some("Holding".to_string()),
                barca: Some("Base".to_string()),
                target_percent: Some(50.0),
                current_quantity: Some(1.0),
                last_price: None,
                notes: None,
                asset_class: "crypto".to_string(),
                created_at: None,
            },
            WalletAllocation {
                id: None,
                symbol: "HGRU11".to_string(),
                group_name: Some("FIIs".to_string()),
                barca: Some("Real Estate".to_string()),
                target_percent: Some(50.0),
                current_quantity: Some(10.0),
                last_price: None,
                notes: None,
                asset_class: "br-equities".to_string(),
                created_at: None,
            },
        ];

        repo.bulk_insert_wallet_allocations(&rows).await.unwrap();

        let current = repo.fetch_current_wallet_allocations().await.unwrap();
        assert_eq!(current.len(), 2);
        assert!(
            current
                .iter()
                .any(|r| r.symbol == "BTC" && r.asset_class == "crypto")
        );
        assert!(
            current
                .iter()
                .any(|r| r.symbol == "HGRU11" && r.asset_class == "br-equities")
        );
    }

    #[tokio::test]
    async fn wallet_allocations_current_keeps_asset_classes_of_the_same_symbol_separate() {
        // Same symbol, same group/barca, but different asset_class — must
        // NOT be merged into one aggregated row (a real collision risk if
        // the view's PARTITION/GROUP BY forgot asset_class).
        let repo = in_memory_repo().await;
        let base = WalletAllocation {
            id: None,
            symbol: "IVV".to_string(),
            group_name: Some("Index".to_string()),
            barca: Some("Indices".to_string()),
            target_percent: Some(20.0),
            current_quantity: Some(5.0),
            last_price: None,
            notes: None,
            asset_class: "us-indices".to_string(),
            created_at: None,
        };
        let same_symbol_different_class = WalletAllocation {
            asset_class: "crypto".to_string(),
            current_quantity: Some(1.0),
            ..base.clone()
        };

        repo.insert_wallet_allocation(&base).await.unwrap();
        repo.insert_wallet_allocation(&same_symbol_different_class)
            .await
            .unwrap();

        let current = repo.fetch_current_wallet_allocations().await.unwrap();
        let ivv_rows: Vec<_> = current.iter().filter(|r| r.symbol == "IVV").collect();
        assert_eq!(
            ivv_rows.len(),
            2,
            "each asset_class should be its own row, not merged"
        );
    }

    fn target(barca: &str, target_percent: f64) -> BarcaTargetInput<'_> {
        BarcaTargetInput {
            barca,
            target_percent,
        }
    }

    #[tokio::test]
    async fn replace_barca_targets_round_trips() {
        let repo = in_memory_repo().await;
        let targets = vec![target("Base", 60.0), target("Altcoins", 40.0)];

        repo.replace_barca_targets("BullMarket", &targets, None)
            .await
            .unwrap();

        let fetched = repo.fetch_barca_targets("BullMarket").await.unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(
            fetched
                .iter()
                .find(|t| t.barca == "Base")
                .unwrap()
                .target_percent,
            60.0
        );
    }

    #[tokio::test]
    async fn replace_barca_targets_drops_barcas_not_included_in_the_new_set() {
        let repo = in_memory_repo().await;
        repo.replace_barca_targets(
            "BullMarket",
            &[target("Base", 60.0), target("Altcoins", 40.0)],
            None,
        )
        .await
        .unwrap();

        // Resubmitting without "Altcoins" must remove it, not leave it stale.
        repo.replace_barca_targets("BullMarket", &[target("Base", 100.0)], None)
            .await
            .unwrap();

        let fetched = repo.fetch_barca_targets("BullMarket").await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].barca, "Base");
    }

    #[tokio::test]
    async fn replace_barca_targets_does_not_touch_other_markets() {
        let repo = in_memory_repo().await;
        repo.replace_barca_targets("BullMarket", &[target("Base", 100.0)], None)
            .await
            .unwrap();
        repo.replace_barca_targets(
            "BearMarket",
            &[target("Base", 50.0), target("Caixa", 50.0)],
            None,
        )
        .await
        .unwrap();
        // A barca with no crypto Bull/Bear cycle (e.g. IBOVE) just lives
        // under whatever market string a manager chooses — "default" here
        // is only a name, not a special always-active scope.
        repo.replace_barca_targets("default", &[target("IBOVE", 100.0)], None)
            .await
            .unwrap();

        assert_eq!(
            repo.fetch_barca_targets("BullMarket").await.unwrap().len(),
            1
        );
        assert_eq!(
            repo.fetch_barca_targets("BearMarket").await.unwrap().len(),
            2
        );
        assert_eq!(repo.fetch_barca_targets("default").await.unwrap().len(), 1);
    }
}
