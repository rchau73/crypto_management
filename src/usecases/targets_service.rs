use crate::domain::models::{AssetClass, WalletAllocation};
use crate::domain::repository::{
    BarcaTargetInput, BarcaTargetRepo, HistoryRepo, PortfolioTargetInput, PortfolioTargetRepo,
};
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

/// Two floats that are supposed to sum to 100 rarely land on exactly
/// 100.0 (e.g. three rows of 33.34/33.33/33.33) — this is slack for that,
/// not a policy that "close enough" targets are fine.
const PERCENT_TOLERANCE: f64 = 0.01;

#[derive(Debug)]
pub enum TargetsServiceError {
    /// The submitted targets don't sum to 100% (within floating-point
    /// tolerance) — the whole save is rejected, nothing is written.
    PercentSumMismatch {
        sum: f64,
    },
    InvalidAssetClass(String),
    Repo(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for TargetsServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetsServiceError::PercentSumMismatch { sum } => {
                write!(f, "target percentages must sum to 100%, got {sum:.2}%")
            }
            TargetsServiceError::InvalidAssetClass(c) => write!(f, "invalid asset_class: {c}"),
            TargetsServiceError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TargetsServiceError {}

impl From<Box<dyn std::error::Error + Send + Sync>> for TargetsServiceError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        TargetsServiceError::Repo(e)
    }
}

fn sums_to_100(values: impl Iterator<Item = f64>) -> Result<(), TargetsServiceError> {
    let sum: f64 = values.sum();
    if (sum - 100.0).abs() > PERCENT_TOLERANCE {
        return Err(TargetsServiceError::PercentSumMismatch { sum });
    }
    Ok(())
}

/// Backs the "editable table + single Save button" admin UI for both the
/// portfolio-allocation targets and the BARCA targets. Both are "replace
/// the full set in one atomic transaction" operations — the UI always
/// submits its complete current state, not a diff — validated up front so
/// a bad submission never touches the DB at all, not even partially.
pub struct TargetsService {
    pub repo: Arc<dyn HistoryRepo>,
    pub barca_target_repo: Arc<dyn BarcaTargetRepo>,
    pub portfolio_target_repo: Arc<dyn PortfolioTargetRepo>,
}

impl TargetsService {
    pub fn new(
        repo: Arc<dyn HistoryRepo>,
        barca_target_repo: Arc<dyn BarcaTargetRepo>,
        portfolio_target_repo: Arc<dyn PortfolioTargetRepo>,
    ) -> Self {
        Self {
            repo,
            barca_target_repo,
            portfolio_target_repo,
        }
    }

    /// Replaces the whole `portfolio_targets` table in one atomic
    /// transaction — target_percent is asset-level configuration, not
    /// ledger history (see migrations/0008 for why it doesn't live on the
    /// append-only `wallet_allocations` ledger), so this is a plain
    /// replace, not an insert, and a target edit always applies outright.
    /// Every row's `target_percent`, across the whole portfolio, must sum
    /// to 100%.
    ///
    /// A genuinely new symbol (no existing `wallet_allocations` entry) also
    /// gets a seed ledger row for its starting quantity/notes. An existing
    /// symbol's quantity/notes are never touched here, regardless of what a
    /// caller submits for them — they're managed by CSV import / Update
    /// Prices — which also means this can never fall into the doubling
    /// trap of resubmitting an aggregated quantity/notes value (a real bug
    /// this design replaced; see the git history of this file).
    pub async fn save_portfolio_targets(
        &self,
        rows: Vec<WalletAllocation>,
        updated_by: Option<i64>,
    ) -> Result<(), TargetsServiceError> {
        for row in &rows {
            if AssetClass::parse(&row.asset_class).is_none() {
                return Err(TargetsServiceError::InvalidAssetClass(
                    row.asset_class.clone(),
                ));
            }
        }
        sums_to_100(rows.iter().map(|r| r.target_percent.unwrap_or(0.0)))?;

        let target_inputs: Vec<PortfolioTargetInput> = rows
            .iter()
            .map(|r| PortfolioTargetInput {
                symbol: &r.symbol,
                group_name: r.group_name.as_deref().unwrap_or(""),
                barca: r.barca.as_deref().unwrap_or(""),
                asset_class: &r.asset_class,
                target_percent: r.target_percent.unwrap_or(0.0),
            })
            .collect();
        self.portfolio_target_repo
            .replace_portfolio_targets(&target_inputs, updated_by)
            .await?;

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
        let new_wallet_rows: Vec<WalletAllocation> = rows
            .into_iter()
            .filter(|r| {
                let key = (
                    r.symbol.clone(),
                    r.group_name.clone().unwrap_or_default(),
                    r.barca.clone().unwrap_or_default(),
                    r.asset_class.clone(),
                );
                !existing_keys.contains(&key)
            })
            .map(|r| WalletAllocation {
                id: None,
                symbol: r.symbol,
                group_name: r.group_name,
                barca: r.barca,
                target_percent: None,
                current_quantity: r.current_quantity,
                last_price: None,
                notes: r.notes,
                asset_class: r.asset_class,
                created_at: None,
            })
            .collect();
        if !new_wallet_rows.is_empty() {
            self.repo
                .bulk_insert_wallet_allocations(&new_wallet_rows)
                .await?;
        }

        Ok(())
    }

    /// Replaces every barca target for one market profile in one atomic
    /// transaction (a barca left out is deleted). Every target for that
    /// market must sum to 100% — see migrations/0006 for why there's no
    /// merging across market profiles.
    pub async fn save_barca_targets(
        &self,
        market: &str,
        targets: &[BarcaTargetInput<'_>],
        updated_by: Option<i64>,
    ) -> Result<(), TargetsServiceError> {
        sums_to_100(targets.iter().map(|t| t.target_percent))?;
        self.barca_target_repo
            .replace_barca_targets(market, targets, updated_by)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::sqlite::SqliteRepo;
    use sqlx::SqlitePool;

    async fn in_memory_service() -> TargetsService {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = Arc::new(SqliteRepo::new(pool));
        TargetsService::new(repo.clone(), repo.clone(), repo)
    }

    fn row(symbol: &str, target: f64) -> WalletAllocation {
        WalletAllocation {
            id: None,
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            target_percent: Some(target),
            current_quantity: Some(1.0),
            last_price: None,
            notes: None,
            asset_class: "crypto".to_string(),
            created_at: None,
        }
    }

    #[tokio::test]
    async fn save_portfolio_targets_rejects_a_sum_below_100_and_writes_nothing() {
        let service = in_memory_service().await;
        let rows = vec![row("BTC", 50.0), row("ETH", 30.0)]; // sums to 80

        let err = service
            .save_portfolio_targets(rows, None)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            TargetsServiceError::PercentSumMismatch { .. }
        ));

        let targets = service
            .portfolio_target_repo
            .fetch_portfolio_targets()
            .await
            .unwrap();
        assert!(
            targets.is_empty(),
            "a rejected save must not write anything"
        );
        let current = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert!(current.is_empty());
    }

    #[tokio::test]
    async fn save_portfolio_targets_rejects_an_unknown_asset_class_and_writes_nothing() {
        let service = in_memory_service().await;
        let mut bad_row = row("HGRU11", 100.0);
        bad_row.asset_class = "not-a-real-class".to_string();

        let err = service
            .save_portfolio_targets(vec![bad_row], None)
            .await
            .unwrap_err();
        assert!(matches!(err, TargetsServiceError::InvalidAssetClass(_)));

        let current = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert!(current.is_empty());
    }

    #[tokio::test]
    async fn editing_the_target_of_an_existing_multi_source_asset_never_touches_wallet_allocations()
    {
        // Regression coverage for a real bug: target_percent used to live
        // on wallet_allocations itself, so "editing" it meant appending a
        // new ledger row — which, for a symbol with several distinct-notes
        // source rows (holdings split across wallets), either doubled its
        // quantity (if the aggregated quantity/notes were resubmitted) or
        // lost the edit entirely (target_percent aggregated via MAX/latest
        // across partitions, both of which turned out fragile). Now a
        // target edit only ever touches portfolio_targets — it structurally
        // cannot perturb wallet_allocations for a symbol that already has
        // ledger entries.
        let service = in_memory_service().await;
        let base = row("BTC", 50.0);
        service
            .repo
            .bulk_insert_wallet_allocations(&[
                WalletAllocation {
                    current_quantity: Some(1.0),
                    notes: Some("Binance".to_string()),
                    ..base.clone()
                },
                WalletAllocation {
                    current_quantity: Some(2.0),
                    notes: Some("Ledger".to_string()),
                    ..base.clone()
                },
            ])
            .await
            .unwrap();

        let aggregated = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated[0].current_quantity, Some(3.0)); // 1.0 + 2.0

        let mut edited = aggregated[0].clone();
        edited.target_percent = Some(100.0);
        service
            .save_portfolio_targets(vec![edited], None)
            .await
            .unwrap();

        let after = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(
            after[0].current_quantity,
            Some(3.0),
            "quantity must be completely unaffected by a target-only edit"
        );
        assert_eq!(after[0].target_percent, Some(100.0));

        let history = service
            .repo
            .fetch_wallet_allocation_history("BTC")
            .await
            .unwrap();
        assert_eq!(
            history.len(),
            2,
            "no new wallet_allocations row should have been inserted for an existing symbol"
        );
    }

    #[tokio::test]
    async fn save_portfolio_targets_seeds_a_wallet_allocations_row_for_a_brand_new_symbol() {
        let service = in_memory_service().await;
        service
            .save_portfolio_targets(vec![row("HGRU11", 100.0)], None)
            .await
            .unwrap();

        let history = service
            .repo
            .fetch_wallet_allocation_history("HGRU11")
            .await
            .unwrap();
        assert_eq!(
            history.len(),
            1,
            "a genuinely new symbol needs a seed ledger row"
        );
        assert_eq!(history[0].current_quantity, Some(1.0)); // from the row() fixture
    }

    #[tokio::test]
    async fn save_portfolio_targets_rejects_a_sum_above_100() {
        let service = in_memory_service().await;
        let rows = vec![row("BTC", 60.0), row("ETH", 60.0)]; // sums to 120

        let err = service
            .save_portfolio_targets(rows, None)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            TargetsServiceError::PercentSumMismatch { .. }
        ));
    }

    #[tokio::test]
    async fn save_portfolio_targets_accepts_and_persists_an_exact_100_sum() {
        let service = in_memory_service().await;
        let rows = vec![row("BTC", 60.0), row("ETH", 40.0)];

        service.save_portfolio_targets(rows, None).await.unwrap();

        let current = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        assert_eq!(current.len(), 2);
    }

    #[tokio::test]
    async fn save_portfolio_targets_tolerates_floating_point_rounding() {
        let service = in_memory_service().await;
        // 33.34 + 33.33 + 33.33 = 100.00 exactly, but a three-way split is
        // exactly the kind of value a manager would type that risks
        // landing a hair off 100 in f64 math.
        let rows = vec![row("BTC", 33.34), row("ETH", 33.33), row("SOL", 33.33)];

        service.save_portfolio_targets(rows, None).await.unwrap();
    }

    #[tokio::test]
    async fn save_portfolio_targets_lowering_an_existing_target_always_applies() {
        let service = in_memory_service().await;
        service
            .save_portfolio_targets(vec![row("BTC", 60.0), row("ETH", 40.0)], None)
            .await
            .unwrap();

        service
            .save_portfolio_targets(vec![row("BTC", 30.0), row("ETH", 70.0)], None)
            .await
            .unwrap();

        let current = service
            .repo
            .fetch_current_wallet_allocations()
            .await
            .unwrap();
        let btc = current.iter().find(|r| r.symbol == "BTC").unwrap();
        assert_eq!(btc.target_percent, Some(30.0));
    }

    #[tokio::test]
    async fn save_barca_targets_rejects_a_bad_sum_and_writes_nothing() {
        let service = in_memory_service().await;
        let targets = [BarcaTargetInput {
            barca: "Base",
            target_percent: 70.0,
        }];

        let err = service
            .save_barca_targets("BullMarket", &targets, None)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            TargetsServiceError::PercentSumMismatch { .. }
        ));

        let fetched = service
            .barca_target_repo
            .fetch_barca_targets("BullMarket")
            .await
            .unwrap();
        assert!(fetched.is_empty());
    }

    #[tokio::test]
    async fn save_barca_targets_accepts_and_replaces_the_full_set_for_that_market() {
        let service = in_memory_service().await;
        let first = [BarcaTargetInput {
            barca: "Base",
            target_percent: 100.0,
        }];
        service
            .save_barca_targets("BullMarket", &first, None)
            .await
            .unwrap();

        let second = [
            BarcaTargetInput {
                barca: "Base",
                target_percent: 60.0,
            },
            BarcaTargetInput {
                barca: "IBOVE",
                target_percent: 40.0,
            },
        ];
        service
            .save_barca_targets("BullMarket", &second, None)
            .await
            .unwrap();

        let fetched = service
            .barca_target_repo
            .fetch_barca_targets("BullMarket")
            .await
            .unwrap();
        assert_eq!(
            fetched.len(),
            2,
            "the first save's set must be replaced, not merged"
        );
    }
}
