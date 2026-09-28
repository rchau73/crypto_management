use crate::domain::models::{
    AssetSnapshot, BarcaSnapshot, GroupSnapshot, TotalSnapshot, WalletAllocation,
};
use crate::domain::repository::{HistoryRepo, PortfolioTargetInput, PortfolioTargetRepo};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

pub struct HistoryService {
    pub repo: Arc<dyn HistoryRepo>,
    pub portfolio_target_repo: Arc<dyn PortfolioTargetRepo>,
}

impl HistoryService {
    pub fn new(
        repo: Arc<dyn HistoryRepo>,
        portfolio_target_repo: Arc<dyn PortfolioTargetRepo>,
    ) -> Self {
        Self {
            repo,
            portfolio_target_repo,
        }
    }

    pub async fn persist_snapshots(
        &self,
        ts: DateTime<Utc>,
        per_asset: &[Value],
        per_group: &[Value],
        per_barca: &[Value],
        total_value: f64,
    ) {
        for a in per_asset {
            let snap = AssetSnapshot {
                id: None,
                timestamp: ts.to_rfc3339(),
                symbol: a
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                group_name: a
                    .get("group")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                barca: a
                    .get("barca")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                price: a.get("price").and_then(|v| v.as_f64()),
                current_quantity: a.get("current_quantity").and_then(|v| v.as_f64()),
                value: a.get("value").and_then(|v| v.as_f64()),
                target_percent: a.get("target_percent").and_then(|v| v.as_f64()),
                current_percent: a.get("current_percent").and_then(|v| v.as_f64()),
                market_cap: a.get("market_cap").and_then(|v| v.as_f64()),
                fdv: a.get("fdv").and_then(|v| v.as_f64()),
                volume_24h: a.get("volume_24h").and_then(|v| v.as_f64()),
                percent_change_24h: a.get("percent_change_24h").and_then(|v| v.as_f64()),
                percent_change_7d: a.get("percent_change_7d").and_then(|v| v.as_f64()),
                extra: None,
                created_at: None,
            };
            if let Err(e) = self.repo.insert_asset_snapshot(&snap).await {
                tracing::error!(error = %e, symbol = %snap.symbol, "Failed to insert asset snapshot");
            }
        }

        for g in per_group {
            let snap = GroupSnapshot {
                id: None,
                timestamp: ts.to_rfc3339(),
                group_name: g
                    .get("group")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                value: g.get("value").and_then(|v| v.as_f64()),
                current_percent: g.get("current_percent").and_then(|v| v.as_f64()),
                target_percent: g.get("target_percent").and_then(|v| v.as_f64()),
                extra: None,
                created_at: None,
            };
            if let Err(e) = self.repo.insert_group_snapshot(&snap).await {
                tracing::error!(error = %e, group = %snap.group_name, "Failed to insert group snapshot");
            }
        }

        for b in per_barca {
            let snap = BarcaSnapshot {
                id: None,
                timestamp: ts.to_rfc3339(),
                barca: b
                    .get("barca")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                value: b.get("value").and_then(|v| v.as_f64()),
                current_percent: b.get("current_percent").and_then(|v| v.as_f64()),
                target_percent: b.get("target_percent").and_then(|v| v.as_f64()),
                extra: None,
                created_at: None,
            };
            if let Err(e) = self.repo.insert_barca_snapshot(&snap).await {
                tracing::error!(error = %e, barca = %snap.barca, "Failed to insert barca snapshot");
            }
        }

        if let Err(e) = self
            .repo
            .insert_total_snapshot(&TotalSnapshot {
                id: None,
                timestamp: ts.to_rfc3339(),
                total_value: Some(total_value),
                extra: None,
                created_at: None,
            })
            .await
        {
            tracing::error!(error = %e, "Failed to insert total snapshot");
        }
    }

    pub async fn fetch_history(
        &self,
        level: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        match level {
            "assets" => {
                let rows = self.repo.fetch_assets(None, None).await?;
                let out: Vec<serde_json::Value> = rows
                    .into_iter()
                    .map(|r| {
                        serde_json::json!({
                            "timestamp": r.timestamp,
                            "symbol": r.symbol,
                            "group": r.group_name,
                            "barca": r.barca,
                            "current_quantity": r.current_quantity,
                            "price": r.price,
                            "value": r.value,
                            "target_percent": r.target_percent,
                            "current_percent": r.current_percent,
                            "deviation": r.deviation_percent,
                            "value_deviation": r.value_deviation
                        })
                    })
                    .collect();
                Ok(serde_json::json!({"level": "assets", "rows": out}))
            }
            "barca" => {
                let rows = self.repo.fetch_barca(None, None).await?;
                let out: Vec<serde_json::Value> = rows
                    .into_iter()
                    .map(|r| {
                        serde_json::json!({
                            "timestamp": r.timestamp,
                            "barca": r.barca,
                            "value": r.value,
                            "current_percent": r.current_percent,
                            "target_percent": r.target_percent,
                            "deviation": r.deviation_percent
                        })
                    })
                    .collect();
                Ok(serde_json::json!({"level": "barca", "rows": out}))
            }
            "groups" => {
                let rows = self.repo.fetch_groups(None, None).await?;
                let out: Vec<serde_json::Value> = rows
                    .into_iter()
                    .map(|r| {
                        serde_json::json!({
                            "timestamp": r.timestamp,
                            "group": r.group_name,
                            "value": r.value,
                            "current_percent": r.current_percent,
                            "target_percent": r.target_percent,
                            "deviation": r.deviation_percent
                        })
                    })
                    .collect();
                Ok(serde_json::json!({"level": "groups", "rows": out}))
            }
            _ => {
                let rows = self.repo.fetch_totals(None, None).await?;
                let out: Vec<serde_json::Value> = rows.into_iter().map(|r| serde_json::json!({"timestamp": r.timestamp, "total_value": r.total_value})).collect();
                Ok(serde_json::json!({"level": "totals", "rows": out}))
            }
        }
    }

    /// The DB is authoritative once it has data for a `(symbol, group,
    /// barca, asset_class)` combination — CSV import only seeds
    /// combinations that don't already exist; it never overwrites an
    /// existing target/quantity. A brand-new row with no target specified
    /// defaults to 0% (it exists, but contributes nothing, until someone
    /// sets a real target via the editor).
    pub async fn import_wallet_allocations_from_path(
        &self,
        path: &str,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let existing = self.repo.fetch_current_wallet_allocations().await?;
        let existing_keys: HashSet<(String, String, String, String)> = existing
            .iter()
            .map(|w| {
                (
                    w.symbol.clone(),
                    w.group_name.clone().unwrap_or_default(),
                    w.barca.clone().unwrap_or_default(),
                    w.asset_class.clone(),
                )
            })
            .collect();

        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .flexible(true)
            .has_headers(true)
            .from_path(path)?;
        let mut count = 0usize;
        // (symbol, group_name, barca, asset_class, target_percent) for every
        // new row — seeded into portfolio_targets after the loop, via
        // insert-if-absent so a pre-existing manager-set target is never
        // overwritten by a re-import.
        let mut new_targets: Vec<(String, String, String, String, f64)> = Vec::new();
        for result in rdr.deserialize::<WalletCsvRow>() {
            let row = result?;
            let asset_class = row
                .asset_class
                .clone()
                .unwrap_or_else(|| "crypto".to_string());
            let group_name = row.group.clone().unwrap_or_default();
            let barca = row.barca.clone().unwrap_or_default();
            let key = (
                row.symbol.clone(),
                group_name.clone(),
                barca.clone(),
                asset_class.clone(),
            );
            if existing_keys.contains(&key) {
                continue; // already tracked in the DB — CSV never overwrites it
            }
            let target_percent = row.target_percent.unwrap_or(0.0);
            new_targets.push((
                row.symbol.clone(),
                group_name,
                barca,
                asset_class.clone(),
                target_percent,
            ));
            let wa = WalletAllocation {
                id: None,
                symbol: row.symbol,
                group_name: row.group,
                barca: row.barca,
                target_percent: None,
                current_quantity: row.current_quantity,
                last_price: row.last_price,
                notes: row.comments,
                asset_class,
                created_at: None,
            };
            self.repo.insert_wallet_allocation(&wa).await?;
            count += 1;
        }

        if !new_targets.is_empty() {
            let inputs: Vec<PortfolioTargetInput> = new_targets
                .iter()
                .map(|(symbol, group_name, barca, asset_class, target_percent)| {
                    PortfolioTargetInput {
                        symbol,
                        group_name,
                        barca,
                        asset_class,
                        target_percent: *target_percent,
                    }
                })
                .collect();
            self.portfolio_target_repo
                .seed_portfolio_targets_if_absent(&inputs)
                .await?;
        }

        Ok(count)
    }
}

#[derive(Debug, Deserialize)]
struct WalletCsvRow {
    symbol: String,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    barca: Option<String>,
    #[serde(default)]
    target_percent: Option<f64>,
    #[serde(default)]
    current_quantity: Option<f64>,
    #[serde(default)]
    last_price: Option<f64>,
    #[serde(default, alias = "comments")]
    comments: Option<String>,
    #[serde(default)]
    asset_class: Option<String>,
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use crate::infra::sqlite::repo::SqliteRepo;
    use sqlx::SqlitePool;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    async fn in_memory_service() -> HistoryService {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        HistoryService::new(
            Arc::new(SqliteRepo::new(pool.clone())),
            Arc::new(SqliteRepo::new(pool)),
        )
    }

    /// A CSV file in the OS temp dir, uniquely named per call so parallel
    /// tests never collide, removed automatically when it goes out of scope.
    struct TempCsv(std::path::PathBuf);

    impl TempCsv {
        fn write(contents: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "wallet_import_test_{}_{}.csv",
                std::process::id(),
                n
            ));
            std::fs::File::create(&path)
                .unwrap()
                .write_all(contents.as_bytes())
                .unwrap();
            Self(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for TempCsv {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn existing_row(symbol: &str, asset_class: &str, target: f64) -> WalletAllocation {
        WalletAllocation {
            id: None,
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("A".to_string()),
            target_percent: Some(target),
            current_quantity: Some(1.0),
            last_price: Some(10.0),
            notes: None,
            asset_class: asset_class.to_string(),
            created_at: None,
        }
    }

    #[tokio::test]
    async fn seeds_every_row_on_first_import_into_an_empty_db() {
        let service = in_memory_service().await;
        let csv = TempCsv::write(
            "symbol,group,barca,target_percent,current_quantity,last_price,comments,asset_class\n\
             BTC,Core,A,50,1.0,10.0,seed,crypto\n\
             ETH,Core,A,50,2.0,5.0,seed,crypto\n",
        );

        let inserted = service
            .import_wallet_allocations_from_path(csv.path())
            .await
            .unwrap();
        assert_eq!(inserted, 2);

        let rows = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn does_not_overwrite_an_existing_db_row_on_reimport() {
        let service = in_memory_service().await;
        service
            .repo
            .insert_wallet_allocation(&existing_row("BTC", "crypto", 40.0))
            .await
            .unwrap();
        service
            .portfolio_target_repo
            .seed_portfolio_targets_if_absent(&[PortfolioTargetInput {
                symbol: "BTC",
                group_name: "Core",
                barca: "A",
                asset_class: "crypto",
                target_percent: 40.0,
            }])
            .await
            .unwrap();

        // Same (symbol, group, barca, asset_class) key, different target —
        // the DB value must win, per "local distribution always overcomes
        // the CSV file entries".
        let csv = TempCsv::write(
            "symbol,group,barca,target_percent,current_quantity,last_price,comments,asset_class\n\
             BTC,Core,A,99,1.0,10.0,seed,crypto\n",
        );

        let inserted = service
            .import_wallet_allocations_from_path(csv.path())
            .await
            .unwrap();
        assert_eq!(inserted, 0);

        let rows = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target_percent, Some(40.0));
    }

    #[tokio::test]
    async fn a_new_row_with_no_target_specified_defaults_to_zero() {
        let service = in_memory_service().await;
        let csv = TempCsv::write(
            "symbol,group,barca,target_percent,current_quantity,last_price,comments,asset_class\n\
             XPML11,BR,Core,,10.0,100.0,new asset,br-equities\n",
        );

        let inserted = service
            .import_wallet_allocations_from_path(csv.path())
            .await
            .unwrap();
        assert_eq!(inserted, 1);

        let rows = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target_percent, Some(0.0));
        assert_eq!(rows[0].asset_class, "br-equities");
    }

    #[tokio::test]
    async fn asset_class_defaults_to_crypto_when_the_csv_omits_the_column() {
        let service = in_memory_service().await;
        let csv = TempCsv::write(
            "symbol,group,barca,target_percent,current_quantity,last_price,comments\n\
             BTC,Core,A,50,1.0,10.0,seed\n",
        );

        service
            .import_wallet_allocations_from_path(csv.path())
            .await
            .unwrap();

        let rows = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(rows[0].asset_class, "crypto");
    }

    #[tokio::test]
    async fn same_symbol_group_and_barca_but_a_different_asset_class_is_seeded_independently() {
        let service = in_memory_service().await;
        service
            .repo
            .insert_wallet_allocation(&existing_row("IBOV", "crypto", 20.0))
            .await
            .unwrap();

        // Same (symbol, group, barca) as the row above but a different
        // asset_class — must be treated as a distinct entry, not skipped as
        // "already tracked".
        let csv = TempCsv::write(
            "symbol,group,barca,target_percent,current_quantity,last_price,comments,asset_class\n\
             IBOV,Core,A,30,1.0,10.0,seed,br-equities\n",
        );

        let inserted = service
            .import_wallet_allocations_from_path(csv.path())
            .await
            .unwrap();
        assert_eq!(inserted, 1);

        let rows = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
    }
}
