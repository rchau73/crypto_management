use crate::domain::models::{
    AssetSnapshot, BarcaSnapshot, GroupSnapshot, TotalSnapshot, WalletAllocation,
};
use crate::domain::repository::HistoryRepo;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

pub struct HistoryService {
    pub repo: Arc<dyn HistoryRepo>,
}

impl HistoryService {
    pub fn new(repo: Arc<dyn HistoryRepo>) -> Self {
        Self { repo }
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

    pub async fn import_wallet_allocations_from_path(
        &self,
        path: &str,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .flexible(true)
            .has_headers(true)
            .from_path(path)?;
        let mut count = 0usize;
        for result in rdr.deserialize::<WalletCsvRow>() {
            let row = result?;
            let wa = WalletAllocation {
                id: None,
                symbol: row.symbol,
                group_name: row.group,
                barca: row.barca,
                target_percent: row.target_percent,
                current_quantity: row.current_quantity,
                last_price: row.last_price,
                notes: row.comments,
                created_at: None,
            };
            self.repo.insert_wallet_allocation(&wa).await?;
            count += 1;
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
}
