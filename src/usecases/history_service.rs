//! Reads the recorded price history for the Dashboard charts.

use crate::domain::repository::{RepoResult, SnapshotRepo};
use serde_json::{Value, json};
use std::sync::Arc;

/// Which history table the Dashboard asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryLevel {
    Totals,
    Assets,
    Groups,
    Barca,
}

impl HistoryLevel {
    pub fn parse(s: &str) -> Option<HistoryLevel> {
        match s {
            "totals" => Some(HistoryLevel::Totals),
            "assets" => Some(HistoryLevel::Assets),
            "groups" => Some(HistoryLevel::Groups),
            "barca" => Some(HistoryLevel::Barca),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HistoryLevel::Totals => "totals",
            HistoryLevel::Assets => "assets",
            HistoryLevel::Groups => "groups",
            HistoryLevel::Barca => "barca",
        }
    }
}

pub struct HistoryService {
    snapshots: Arc<dyn SnapshotRepo>,
}

impl HistoryService {
    pub fn new(snapshots: Arc<dyn SnapshotRepo>) -> Self {
        Self { snapshots }
    }

    /// Returns `{"level": "...", "rows": [...]}`.
    pub async fn fetch_history(&self, level: HistoryLevel) -> RepoResult<Value> {
        let rows = match level {
            HistoryLevel::Totals => json!(self.snapshots.fetch_total_history().await?),
            HistoryLevel::Assets => json!(self.snapshots.fetch_asset_history().await?),
            HistoryLevel::Groups => json!(self.snapshots.fetch_group_history().await?),
            HistoryLevel::Barca => json!(self.snapshots.fetch_barca_history().await?),
        };
        Ok(json!({ "level": level.as_str(), "rows": rows }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{AllocationReport, AllocationSnapshot, AssetAllocation};
    use crate::infra::sqlite::test_repo;

    #[test]
    fn level_round_trips_and_rejects_unknown_values() {
        for level in [
            HistoryLevel::Totals,
            HistoryLevel::Assets,
            HistoryLevel::Groups,
            HistoryLevel::Barca,
        ] {
            assert_eq!(HistoryLevel::parse(level.as_str()), Some(level));
        }
        assert_eq!(HistoryLevel::parse("weekly"), None);
    }

    #[tokio::test]
    async fn asset_rows_keep_the_field_names_the_frontend_expects() {
        let repo = test_repo().await;
        repo.record_snapshot(&AllocationSnapshot {
            timestamp: "2026-01-01T00:00:00Z".into(),
            report: AllocationReport {
                total_value: 10.0,
                per_asset: vec![AssetAllocation {
                    symbol: "BTC".into(),
                    group: "Core".into(),
                    barca: "Base".into(),
                    price: 10.0,
                    current_quantity: 1.0,
                    value: 10.0,
                    target_percent: 60.0,
                    current_percent: 100.0,
                    deviation: 40.0,
                }],
                ..Default::default()
            },
            asset_notes: Default::default(),
        })
        .await
        .unwrap();

        let history = HistoryService::new(repo)
            .fetch_history(HistoryLevel::Assets)
            .await
            .unwrap();

        assert_eq!(history["level"], "assets");
        let row = &history["rows"][0];
        assert_eq!(row["group"], "Core");
        assert!(row.get("deviation").is_some());
        assert!(row.get("value_deviation").is_some());
    }
}
