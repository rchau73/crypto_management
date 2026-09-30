//! Editing portfolio targets, BARCA targets and single-source quantities
//! (the Manager "Portfolio Targets" / "BARCA Targets" tabs).
//!
//! Every save is validated up front, so a bad request never touches the
//! database, and then written in a single transaction.

use crate::domain::models::{
    BarcaTarget, LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PositionKey, WalletPosition,
};
use crate::domain::repository::{BarcaTargetRepo, PortfolioRepo, RepoError};
use crate::usecases::validation::{
    check_asset_class, check_not_blank, check_percent, check_quantity, check_sums_to_100,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub enum TargetsError {
    /// The request itself is wrong (HTTP 400). The message says why.
    Invalid(String),
    /// The position to correct doesn't exist (HTTP 404).
    NotFound(String),
    Repo(RepoError),
}

impl fmt::Display for TargetsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetsError::Invalid(message) | TargetsError::NotFound(message) => {
                write!(f, "{message}")
            }
            TargetsError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TargetsError {}

impl From<RepoError> for TargetsError {
    fn from(e: RepoError) -> Self {
        TargetsError::Repo(e)
    }
}

impl From<String> for TargetsError {
    fn from(message: String) -> Self {
        TargetsError::Invalid(message)
    }
}

/// One row of the "Save All" request from the Portfolio Targets table.
#[derive(Debug, Clone, Deserialize)]
pub struct TargetRow {
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub asset_class: String,
    pub target_percent: f64,
    /// Only used when this row is a brand-new position.
    #[serde(default)]
    pub current_quantity: Option<f64>,
    /// Only used when this row is a brand-new position.
    #[serde(default)]
    pub notes: Option<String>,
}

impl TargetRow {
    fn key(&self) -> PositionKey {
        PositionKey {
            symbol: self.symbol.clone(),
            group_name: self.group_name.clone().unwrap_or_default(),
            barca: self.barca.clone().unwrap_or_default(),
            asset_class: self.asset_class.clone(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        check_not_blank("symbol", &self.symbol)?;
        check_asset_class(&self.asset_class)?;
        check_percent(self.target_percent)?;
        if let Some(quantity) = self.current_quantity {
            check_quantity(quantity)?;
        }
        Ok(())
    }
}

/// A request to fix the quantity of one position.
#[derive(Debug, Clone, Deserialize)]
pub struct QuantityCorrection {
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub asset_class: String,
    pub current_quantity: f64,
}

impl QuantityCorrection {
    fn key(&self) -> PositionKey {
        PositionKey {
            symbol: self.symbol.clone(),
            group_name: self.group_name.clone().unwrap_or_default(),
            barca: self.barca.clone().unwrap_or_default(),
            asset_class: self.asset_class.clone(),
        }
    }
}

pub struct TargetsService {
    portfolio: Arc<dyn PortfolioRepo>,
    barca_targets: Arc<dyn BarcaTargetRepo>,
}

impl TargetsService {
    pub fn new(portfolio: Arc<dyn PortfolioRepo>, barca_targets: Arc<dyn BarcaTargetRepo>) -> Self {
        Self {
            portfolio,
            barca_targets,
        }
    }

    pub async fn list_positions(&self) -> Result<Vec<WalletPosition>, TargetsError> {
        Ok(self.portfolio.fetch_positions().await?)
    }

    /// Replaces every portfolio target with `rows` (they must sum to 100%).
    ///
    /// A row for a position that doesn't exist yet also creates it, with
    /// the row's starting quantity/notes. For an existing position the
    /// submitted quantity/notes are ignored: the table shows a *sum* over
    /// sources, and writing that sum back as a new source would double the
    /// holding.
    pub async fn save_portfolio_targets(
        &self,
        rows: Vec<TargetRow>,
        updated_by: Option<i64>,
    ) -> Result<(), TargetsError> {
        let mut seen = HashSet::new();
        for row in &rows {
            row.validate()?;
            if !seen.insert(row.key()) {
                return Err(TargetsError::Invalid(format!(
                    "duplicate row for {} (same group, BARCA and asset class)",
                    row.symbol
                )));
            }
        }
        check_sums_to_100(rows.iter().map(|r| r.target_percent))?;

        let existing: HashSet<PositionKey> = self
            .portfolio
            .fetch_positions()
            .await?
            .iter()
            .map(|p| p.key())
            .collect();

        let targets: Vec<NewPortfolioTarget> = rows
            .iter()
            .map(|row| NewPortfolioTarget {
                key: row.key(),
                target_percent: row.target_percent,
            })
            .collect();
        let new_entries: Vec<LedgerEntry> = rows
            .into_iter()
            .filter(|row| !existing.contains(&row.key()))
            .map(|row| LedgerEntry {
                symbol: row.symbol,
                group_name: row.group_name,
                barca: row.barca,
                asset_class: row.asset_class,
                current_quantity: row.current_quantity.unwrap_or(0.0),
                last_price: None,
                notes: row.notes,
            })
            .collect();

        self.portfolio
            .replace_targets(&targets, &new_entries, updated_by)
            .await?;
        Ok(())
    }

    /// Sets the quantity of a position that has exactly one source, by
    /// appending a new ledger row for that source (which supersedes the old
    /// one). A multi-source position is refused: there is no way to know
    /// which source the new number belongs to.
    pub async fn correct_quantity(
        &self,
        correction: QuantityCorrection,
    ) -> Result<(), TargetsError> {
        check_asset_class(&correction.asset_class)?;
        check_quantity(correction.current_quantity)?;

        let key = correction.key();
        let position = self
            .portfolio
            .fetch_positions()
            .await?
            .into_iter()
            .find(|p| p.key() == key)
            .ok_or_else(|| {
                TargetsError::NotFound(format!("no position for {}", correction.symbol))
            })?;
        if position.source_count != 1 {
            return Err(TargetsError::Invalid(format!(
                "{} is held in {} sources; correct each source through a CSV import instead",
                correction.symbol, position.source_count
            )));
        }

        let entry = LedgerEntry {
            symbol: position.symbol,
            group_name: position.group_name,
            barca: position.barca,
            asset_class: position.asset_class,
            current_quantity: correction.current_quantity,
            last_price: position.last_price,
            // The single source's own notes, so the new row replaces that
            // source instead of being added as a second one.
            notes: position.notes,
        };
        self.portfolio.append_ledger_entries(&[entry]).await?;
        Ok(())
    }

    pub async fn list_barca_targets(&self, market: &str) -> Result<Vec<BarcaTarget>, TargetsError> {
        Ok(self.barca_targets.fetch_barca_targets(market).await?)
    }

    /// Replaces every BARCA target of `market` (they must sum to 100%).
    pub async fn save_barca_targets(
        &self,
        market: &str,
        targets: &[NewBarcaTarget],
        updated_by: Option<i64>,
    ) -> Result<(), TargetsError> {
        check_not_blank("market", market)?;
        let mut seen = HashSet::new();
        for target in targets {
            check_not_blank("barca", &target.barca)?;
            check_percent(target.target_percent)?;
            if !seen.insert(target.barca.as_str()) {
                return Err(TargetsError::Invalid(format!(
                    "duplicate BARCA: {}",
                    target.barca
                )));
            }
        }
        check_sums_to_100(targets.iter().map(|t| t.target_percent))?;

        self.barca_targets
            .replace_barca_targets(market, targets, updated_by)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::sqlite::repo::test_support::{barca, entry};
    use crate::infra::sqlite::{SqliteRepo, test_repo};

    async fn setup() -> (Arc<SqliteRepo>, TargetsService) {
        let repo = test_repo().await;
        (repo.clone(), TargetsService::new(repo.clone(), repo))
    }

    fn row(symbol: &str, target: f64) -> TargetRow {
        TargetRow {
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            asset_class: "crypto".to_string(),
            target_percent: target,
            current_quantity: Some(1.0),
            notes: None,
        }
    }

    fn correction(symbol: &str, quantity: f64) -> QuantityCorrection {
        QuantityCorrection {
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            asset_class: "crypto".to_string(),
            current_quantity: quantity,
        }
    }

    async fn assert_rejected_and_nothing_written(
        repo: &SqliteRepo,
        service: &TargetsService,
        rows: Vec<TargetRow>,
    ) -> String {
        let err = service
            .save_portfolio_targets(rows, None)
            .await
            .unwrap_err();
        assert!(matches!(err, TargetsError::Invalid(_)), "{err:?}");
        assert!(repo.fetch_positions().await.unwrap().is_empty());
        err.to_string()
    }

    // --- save_portfolio_targets ------------------------------------------

    #[tokio::test]
    async fn rejects_a_sum_other_than_100() {
        let (repo, service) = setup().await;
        let msg = assert_rejected_and_nothing_written(
            &repo,
            &service,
            vec![row("BTC", 50.0), row("ETH", 30.0)],
        )
        .await;
        assert!(msg.contains("80.00%"));
        assert_rejected_and_nothing_written(
            &repo,
            &service,
            vec![row("BTC", 60.0), row("ETH", 60.0)],
        )
        .await;
    }

    #[tokio::test]
    async fn rejects_invalid_rows() {
        let (repo, service) = setup().await;

        let mut bad_class = row("HGRU11", 100.0);
        bad_class.asset_class = "not-a-class".to_string();
        let mut negative_quantity = row("BTC", 100.0);
        negative_quantity.current_quantity = Some(-1.0);
        let blank_symbol = row("  ", 100.0);

        for bad in [bad_class, negative_quantity, blank_symbol] {
            assert_rejected_and_nothing_written(&repo, &service, vec![bad]).await;
        }
        // Negative targets can't be used to game the 100% sum.
        assert_rejected_and_nothing_written(
            &repo,
            &service,
            vec![row("BTC", -50.0), row("ETH", 150.0)],
        )
        .await;
    }

    #[tokio::test]
    async fn rejects_duplicate_rows_as_a_client_error_not_a_db_error() {
        let (repo, service) = setup().await;
        let msg = assert_rejected_and_nothing_written(
            &repo,
            &service,
            vec![row("BTC", 50.0), row("BTC", 50.0)],
        )
        .await;
        assert!(msg.contains("duplicate"));
    }

    #[tokio::test]
    async fn accepts_a_sum_within_rounding_tolerance() {
        let (_, service) = setup().await;
        service
            .save_portfolio_targets(
                vec![row("BTC", 33.34), row("ETH", 33.33), row("SOL", 33.33)],
                None,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_new_symbol_gets_a_seed_ledger_row_with_its_starting_quantity() {
        let (repo, service) = setup().await;
        service
            .save_portfolio_targets(vec![row("HGRU11", 100.0)], None)
            .await
            .unwrap();

        let positions = repo.fetch_positions().await.unwrap();
        assert_eq!(positions[0].current_quantity, 1.0);
        assert_eq!(positions[0].target_percent, 100.0);
    }

    #[tokio::test]
    async fn editing_a_multi_source_position_never_touches_the_ledger() {
        // Regression: re-submitting the displayed (summed) quantity used to
        // be written back as one more source, doubling the holding.
        let (repo, service) = setup().await;
        repo.append_ledger_entries(&[
            entry("BTC", 1.0, Some("Binance")),
            entry("BTC", 2.0, Some("Ledger")),
        ])
        .await
        .unwrap();

        let mut edited = row("BTC", 100.0);
        edited.current_quantity = Some(3.0);
        edited.notes = Some("Binance | Ledger".to_string());
        service
            .save_portfolio_targets(vec![edited], None)
            .await
            .unwrap();

        let position = &repo.fetch_positions().await.unwrap()[0];
        assert_eq!(position.current_quantity, 3.0);
        assert_eq!(position.target_percent, 100.0);
        assert_eq!(repo.count_ledger_rows("BTC").await, 2);
    }

    #[tokio::test]
    async fn a_later_save_can_lower_a_target() {
        let (repo, service) = setup().await;
        service
            .save_portfolio_targets(vec![row("BTC", 60.0), row("ETH", 40.0)], None)
            .await
            .unwrap();
        service
            .save_portfolio_targets(vec![row("BTC", 30.0), row("ETH", 70.0)], None)
            .await
            .unwrap();
        assert_eq!(repo.target_of("BTC").await, Some(30.0));
    }

    // --- correct_quantity --------------------------------------------------

    #[tokio::test]
    async fn correcting_a_single_source_replaces_its_quantity() {
        let (repo, service) = setup().await;
        repo.append_ledger_entries(&[entry("HGRU11", 10.0, Some("XP"))])
            .await
            .unwrap();

        service
            .correct_quantity(correction("HGRU11", 67.0))
            .await
            .unwrap();

        let position = &repo.fetch_positions().await.unwrap()[0];
        assert_eq!(position.current_quantity, 67.0, "replaced, not added");
        assert_eq!(position.source_count, 1);
    }

    #[tokio::test]
    async fn correcting_a_source_without_notes_works_too() {
        let (repo, service) = setup().await;
        repo.append_ledger_entries(&[entry("BTC", 1.0, None)])
            .await
            .unwrap();

        service
            .correct_quantity(correction("BTC", 2.0))
            .await
            .unwrap();

        assert_eq!(
            repo.fetch_positions().await.unwrap()[0].current_quantity,
            2.0
        );
    }

    #[tokio::test]
    async fn correcting_a_multi_source_position_is_refused() {
        // Includes the NULL-notes case the old " | " check missed.
        let (repo, service) = setup().await;
        repo.append_ledger_entries(&[entry("BTC", 1.0, None), entry("BTC", 2.0, Some("Binance"))])
            .await
            .unwrap();

        let err = service
            .correct_quantity(correction("BTC", 5.0))
            .await
            .unwrap_err();

        assert!(matches!(err, TargetsError::Invalid(_)));
        assert_eq!(
            repo.fetch_positions().await.unwrap()[0].current_quantity,
            3.0
        );
    }

    #[tokio::test]
    async fn correcting_an_unknown_position_is_not_found() {
        let (_, service) = setup().await;
        let err = service
            .correct_quantity(correction("GHOST", 1.0))
            .await
            .unwrap_err();
        assert!(matches!(err, TargetsError::NotFound(_)));
    }

    #[tokio::test]
    async fn correcting_with_a_bad_value_is_refused() {
        let (repo, service) = setup().await;
        repo.append_ledger_entries(&[entry("BTC", 1.0, None)])
            .await
            .unwrap();

        let mut bad_class = correction("BTC", 1.0);
        bad_class.asset_class = "nope".to_string();
        for bad in [correction("BTC", -1.0), bad_class] {
            let err = service.correct_quantity(bad).await.unwrap_err();
            assert!(matches!(err, TargetsError::Invalid(_)));
        }
    }

    // --- save_barca_targets --------------------------------------------------

    #[tokio::test]
    async fn barca_targets_are_validated_before_anything_is_written() {
        let (repo, service) = setup().await;
        let bad_requests: Vec<(&str, Vec<NewBarcaTarget>)> = vec![
            ("BullMarket", vec![barca("Base", 70.0)]),
            ("BullMarket", vec![barca("Base", 50.0), barca("Base", 50.0)]),
            ("BullMarket", vec![barca(" ", 100.0)]),
            ("", vec![barca("Base", 100.0)]),
        ];
        for (market, targets) in bad_requests {
            let err = service
                .save_barca_targets(market, &targets, None)
                .await
                .unwrap_err();
            assert!(matches!(err, TargetsError::Invalid(_)));
        }
        assert!(
            repo.fetch_barca_targets("BullMarket")
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn barca_targets_replace_the_full_set() {
        let (repo, service) = setup().await;
        service
            .save_barca_targets("BullMarket", &[barca("Base", 100.0)], None)
            .await
            .unwrap();
        service
            .save_barca_targets(
                "BullMarket",
                &[barca("Base", 60.0), barca("IBOVE", 40.0)],
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            repo.fetch_barca_targets("BullMarket").await.unwrap().len(),
            2
        );
    }
}
