//! Seeds the portfolio from a wallet CSV (upload, startup bootstrap, or the
//! `import_wallet_allocations` binary).
//!
//! The database wins over the CSV: a (symbol, group, barca, asset_class)
//! that already exists is skipped, never overwritten. Only new positions
//! are added, each with its starting quantity and target.

use crate::domain::models::{LedgerEntry, NewPortfolioTarget, PositionKey};
use crate::domain::repository::{PortfolioRepo, RepoError};
use crate::usecases::validation::{
    check_asset_class, check_not_blank, check_percent, check_quantity,
};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::io::Read;
use std::sync::Arc;

#[derive(Debug)]
pub enum ImportError {
    /// Not readable as CSV at all.
    Csv(csv::Error),
    /// Valid CSV, but without the one column we can't do without.
    MissingSymbolColumn {
        found_columns: Vec<String>,
    },
    /// One message per bad row, e.g. "line 3: quantity must be >= 0".
    /// Nothing is imported when any row is bad.
    InvalidRows(Vec<String>),
    Repo(RepoError),
}

impl ImportError {
    /// True when the file itself is the problem (HTTP 400), false for a
    /// server-side failure (HTTP 500).
    pub fn is_client_error(&self) -> bool {
        !matches!(self, ImportError::Repo(_))
    }
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Csv(e) => write!(f, "invalid CSV: {e}"),
            ImportError::MissingSymbolColumn { found_columns } => write!(
                f,
                "CSV is missing the required \"symbol\" column (found columns: {})",
                if found_columns.is_empty() {
                    "none".to_string()
                } else {
                    found_columns.join(", ")
                }
            ),
            ImportError::InvalidRows(errors) => {
                write!(f, "{} invalid row(s): {}", errors.len(), errors.join("; "))
            }
            ImportError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<csv::Error> for ImportError {
    fn from(e: csv::Error) -> Self {
        ImportError::Csv(e)
    }
}

impl From<RepoError> for ImportError {
    fn from(e: RepoError) -> Self {
        ImportError::Repo(e)
    }
}

/// One line of the wallet CSV. Only `symbol` is required.
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
    notes: Option<String>,
    #[serde(default)]
    asset_class: Option<String>,
}

impl WalletCsvRow {
    fn asset_class(&self) -> String {
        self.asset_class
            .clone()
            .unwrap_or_else(|| "crypto".to_string())
    }

    fn key(&self) -> PositionKey {
        PositionKey {
            symbol: self.symbol.clone(),
            group_name: self.group.clone().unwrap_or_default(),
            barca: self.barca.clone().unwrap_or_default(),
            asset_class: self.asset_class(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        check_not_blank("symbol", &self.symbol)?;
        check_asset_class(&self.asset_class())?;
        if let Some(quantity) = self.current_quantity {
            check_quantity(quantity)?;
        }
        if let Some(percent) = self.target_percent {
            check_percent(percent)?;
        }
        Ok(())
    }
}

/// Reads and validates every row. Returns rows keyed by position; if the
/// same key appears more than once, the last line wins.
fn parse_rows(reader: impl Read) -> Result<BTreeMap<PositionKey, WalletCsvRow>, ImportError> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(reader);

    let headers = csv_reader.headers()?.clone();
    if !headers.iter().any(|h| h == "symbol") {
        return Err(ImportError::MissingSymbolColumn {
            found_columns: headers.iter().map(String::from).collect(),
        });
    }

    let mut rows = BTreeMap::new();
    let mut errors = Vec::new();
    for record in csv_reader.records() {
        let record = record?;
        let line = record.position().map_or(0, |p| p.line());
        let row: WalletCsvRow = record.deserialize(Some(&headers))?;
        match row.validate() {
            Ok(()) => {
                rows.insert(row.key(), row);
            }
            Err(message) => errors.push(format!("line {line}: {message}")),
        }
    }
    if !errors.is_empty() {
        return Err(ImportError::InvalidRows(errors));
    }
    Ok(rows)
}

pub struct WalletImportService {
    portfolio: Arc<dyn PortfolioRepo>,
}

impl WalletImportService {
    pub fn new(portfolio: Arc<dyn PortfolioRepo>) -> Self {
        Self { portfolio }
    }

    /// Imports the positions in `reader` that don't exist yet. Returns how
    /// many were added. All-or-nothing: a bad row means nothing is written.
    pub async fn import_csv(&self, reader: impl Read) -> Result<usize, ImportError> {
        let rows = parse_rows(reader)?;

        let existing: HashSet<PositionKey> = self
            .portfolio
            .fetch_positions()
            .await?
            .iter()
            .map(|p| p.key())
            .collect();

        let mut targets = Vec::new();
        let mut entries = Vec::new();
        for (key, row) in rows {
            if existing.contains(&key) {
                continue; // the database wins over the CSV
            }
            targets.push(NewPortfolioTarget {
                key: key.clone(),
                target_percent: row.target_percent.unwrap_or(0.0),
            });
            entries.push(LedgerEntry {
                symbol: key.symbol,
                group_name: row.group,
                barca: row.barca,
                asset_class: key.asset_class,
                current_quantity: row.current_quantity.unwrap_or(0.0),
                last_price: row.last_price,
                notes: row.notes,
            });
        }

        if !entries.is_empty() {
            self.portfolio.import_positions(&targets, &entries).await?;
        }
        Ok(entries.len())
    }

    pub async fn import_csv_file(&self, path: &str) -> Result<usize, ImportError> {
        let file = std::fs::File::open(path).map_err(|e| ImportError::Csv(e.into()))?;
        self.import_csv(file).await
    }

    /// On first start (empty portfolio), seeds from the CSV at `path`.
    /// Failures are logged, not fatal: the app works without a seed file.
    pub async fn seed_if_empty(&self, path: &str) {
        match self.portfolio.fetch_positions().await {
            Ok(positions) if positions.is_empty() => match self.import_csv_file(path).await {
                Ok(imported) => tracing::info!(path, imported, "Seeded portfolio from CSV"),
                Err(e) => tracing::warn!(path, error = %e, "Could not seed portfolio from CSV"),
            },
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "Could not check portfolio before seeding"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::sqlite::repo::test_support::{entry, target};
    use crate::infra::sqlite::{SqliteRepo, test_repo};

    async fn setup() -> (Arc<SqliteRepo>, WalletImportService) {
        let repo = test_repo().await;
        (repo.clone(), WalletImportService::new(repo))
    }

    const HEADER: &str = "symbol,group,barca,target_percent,current_quantity,notes,asset_class\n";

    #[tokio::test]
    async fn imports_every_row_into_an_empty_portfolio() {
        let (repo, service) = setup().await;
        let csv =
            format!("{HEADER}BTC,Core,Base,50,1.0,seed,crypto\nETH,Core,Base,50,2.0,seed,crypto\n");

        assert_eq!(service.import_csv(csv.as_bytes()).await.unwrap(), 2);
        assert_eq!(repo.fetch_positions().await.unwrap().len(), 2);
        assert_eq!(repo.target_of("BTC").await, Some(50.0));
    }

    #[tokio::test]
    async fn a_repeated_key_is_imported_once() {
        // Regression: pointing the importer at a time-series export (same
        // key on every line) used to insert one ledger row per line.
        let (repo, service) = setup().await;
        let csv = format!(
            "{HEADER}BTC,Core,Base,10,1.0,t1,crypto\nBTC,Core,Base,10,1.0,t2,crypto\nBTC,Core,Base,10,1.0,t3,crypto\n"
        );

        assert_eq!(service.import_csv(csv.as_bytes()).await.unwrap(), 1);
        assert_eq!(repo.count_ledger_rows("BTC").await, 1);
    }

    #[tokio::test]
    async fn an_existing_position_is_never_overwritten() {
        let (repo, service) = setup().await;
        repo.import_positions(&[target("BTC", 40.0)], &[entry("BTC", 1.0, None)])
            .await
            .unwrap();

        let csv = format!("{HEADER}BTC,Core,Base,99,5.0,seed,crypto\n");
        assert_eq!(service.import_csv(csv.as_bytes()).await.unwrap(), 0);
        assert_eq!(repo.target_of("BTC").await, Some(40.0));
        assert_eq!(
            repo.fetch_positions().await.unwrap()[0].current_quantity,
            1.0
        );
    }

    #[tokio::test]
    async fn missing_optional_columns_get_safe_defaults() {
        // Also a regression test: a "quantity" column (instead of
        // "current_quantity") must import as 0, not a row that later fails
        // to decode.
        let (repo, service) = setup().await;
        let csv = "symbol,group,barca,quantity\nASTR,Holding,Altcoins,55762.8\n";

        service.import_csv(csv.as_bytes()).await.unwrap();

        let position = &repo.fetch_positions().await.unwrap()[0];
        assert_eq!(position.current_quantity, 0.0);
        assert_eq!(position.target_percent, 0.0);
        assert_eq!(position.asset_class, "crypto");
    }

    #[tokio::test]
    async fn a_file_without_a_symbol_column_is_a_client_error() {
        let (repo, service) = setup().await;
        let err = service
            .import_csv("timestamp,price\n2025-01-01,100\n".as_bytes())
            .await
            .unwrap_err();

        assert!(err.is_client_error());
        assert!(matches!(err, ImportError::MissingSymbolColumn { .. }));
        assert!(repo.fetch_positions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn bad_rows_are_all_reported_with_line_numbers_and_nothing_is_imported() {
        let (repo, service) = setup().await;
        let csv = format!(
            "{HEADER}BTC,Core,Base,50,1.0,,crypto\nETH,Core,Base,50,-2.0,,crypto\nSOL,Core,Base,150,1.0,,crypto\nXPML11,BR,Base,0,1.0,,stocks\n"
        );

        let err = service.import_csv(csv.as_bytes()).await.unwrap_err();

        let ImportError::InvalidRows(errors) = &err else {
            panic!("expected InvalidRows, got {err:?}");
        };
        assert_eq!(errors.len(), 3);
        assert!(errors[0].starts_with("line 3:"), "{}", errors[0]);
        assert!(errors[1].starts_with("line 4:"));
        assert!(errors[2].contains("asset_class"));
        assert!(err.is_client_error());
        assert!(repo.fetch_positions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_same_key_in_another_asset_class_is_a_new_position() {
        let (repo, service) = setup().await;
        repo.import_positions(&[], &[entry("IBOV", 1.0, None)])
            .await
            .unwrap();

        let csv = format!("{HEADER}IBOV,Core,Base,30,1.0,,br-equities\n");
        assert_eq!(service.import_csv(csv.as_bytes()).await.unwrap(), 1);
        assert_eq!(repo.fetch_positions().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn seed_if_empty_does_nothing_when_the_portfolio_has_data() {
        let (repo, service) = setup().await;
        repo.import_positions(&[], &[entry("BTC", 1.0, None)])
            .await
            .unwrap();

        service.seed_if_empty("/definitely/not/a/file.csv").await;

        assert_eq!(repo.fetch_positions().await.unwrap().len(), 1);
    }
}
