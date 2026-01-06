use crate::domain::models::{
    AllocationRecord, AssetHistoryRow, AssetSnapshot, BarcaHistoryRow, BarcaSnapshot,
    GroupHistoryRow, GroupSnapshot, TotalSnapshot, WalletAllocation,
};
use crate::domain::repository::{HistoryRepo, RepoResult};
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
        let extra_notes = wa.notes.as_ref().map(|s| s.as_str());
        sqlx::query("INSERT INTO wallet_allocations (symbol, group_name, barca, target_percent, current_quantity, last_price, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(&wa.symbol)
            .bind(&wa.group_name)
            .bind(&wa.barca)
            .bind(wa.target_percent)
            .bind(wa.current_quantity)
            .bind(wa.last_price)
            .bind(extra_notes)
            .execute(&self.pool)
            .await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wallet_allocations_current_aggregates_quantities() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteRepo::new(pool.clone());

        let wa1 = WalletAllocation {
            id: None,
            symbol: "BTC".to_string(),
            group_name: Some("Base".to_string()),
            barca: Some("Base".to_string()),
            target_percent: Some(40.0),
            current_quantity: Some(1.0),
            last_price: Some(10.0),
            notes: Some("Ledger".to_string()),
            created_at: None,
        };
        let wa2 = WalletAllocation {
            current_quantity: Some(0.5),
            notes: Some("Binance".to_string()),
            target_percent: Some(0.0),
            ..wa1.clone()
        };

        repo.insert_wallet_allocation(&wa1).await.unwrap();
        repo.insert_wallet_allocation(&wa2).await.unwrap();

        let rows = repo.fetch_current_wallet_allocations().await.unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!((row.current_quantity.unwrap() - 1.5).abs() < f64::EPSILON);
        assert_eq!(row.target_percent.unwrap(), 40.0);
    }
}
