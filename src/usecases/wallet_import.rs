//! Seeds the portfolio from a wallet CSV (upload, startup bootstrap, or the
//! `import_wallet_allocations` binary).
//!
//! The database wins over the CSV: a (symbol, group, barca, asset_class)
//! that already exists is skipped, never overwritten. Only new positions
//! are added, each with its starting quantity and target.
//!
//! A position may span several lines, one per funding source (the `notes`
//! / `comments` column: "Binance", "Ledger Wallet"...). Each source becomes
//! its own ledger row, and the position's target is the sum of its lines'
//! targets (the wallet CSV puts it on one line and 0 on the others).

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

/// One position from the CSV: its target and one row per source.
#[derive(Debug)]
struct CsvPosition {
    target_percent: f64,
    sources: Vec<WalletCsvRow>,
}

/// Reads and validates every row, grouped by position. Within a position,
/// a repeated source (same `notes`, including no notes) keeps only its last
/// line — so a time-series export with the same key on every line imports
/// once instead of once per line.
fn parse_rows(reader: impl Read) -> Result<BTreeMap<PositionKey, CsvPosition>, ImportError> {
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

    let mut sources: BTreeMap<(PositionKey, Option<String>), WalletCsvRow> = BTreeMap::new();
    let mut errors = Vec::new();
    for record in csv_reader.records() {
        let record = record?;
        let line = record.position().map_or(0, |p| p.line());
        let row: WalletCsvRow = record.deserialize(Some(&headers))?;
        match row.validate() {
            Ok(()) => {
                sources.insert((row.key(), row.notes.clone()), row);
            }
            Err(message) => errors.push(format!("line {line}: {message}")),
        }
    }
    if !errors.is_empty() {
        return Err(ImportError::InvalidRows(errors));
    }

    let mut positions: BTreeMap<PositionKey, CsvPosition> = BTreeMap::new();
    for ((key, _notes), row) in sources {
        let position = positions.entry(key).or_insert(CsvPosition {
            target_percent: 0.0,
            sources: Vec::new(),
        });
        position.target_percent += row.target_percent.unwrap_or(0.0);
        position.sources.push(row);
    }
    let errors: Vec<String> = positions
        .iter()
        .filter_map(|(key, position)| {
            check_percent(position.target_percent).err().map(|message| {
                format!(
                    "{} ({}/{}): summed {message}",
                    key.symbol, key.group_name, key.barca
                )
            })
        })
        .collect();
    if !errors.is_empty() {
        return Err(ImportError::InvalidRows(errors));
    }
    Ok(positions)
}

pub struct WalletImportService {
    portfolio: Arc<dyn PortfolioRepo>,
}

impl WalletImportService {
    pub fn new(portfolio: Arc<dyn PortfolioRepo>) -> Self {
        Self { portfolio }
    }

    /// Imports the positions in `reader` that don't exist yet. Returns how
    /// many positions were added. All-or-nothing: a bad row means nothing
    /// is written.
    pub async fn import_csv(&self, reader: impl Read) -> Result<usize, ImportError> {
        let positions = parse_rows(reader)?;

        let existing: HashSet<PositionKey> = self
            .portfolio
            .fetch_positions()
            .await?
            .iter()
            .map(|p| p.key())
            .collect();

        let mut targets = Vec::new();
        let mut entries = Vec::new();
        for (key, position) in positions {
            if existing.contains(&key) {
                continue; // the database wins over the CSV
            }
            targets.push(NewPortfolioTarget {
                key: key.clone(),
                target_percent: position.target_percent,
            });
            for row in position.sources {
                entries.push(LedgerEntry {
                    symbol: key.symbol.clone(),
                    group_name: row.group,
                    barca: row.barca,
                    asset_class: key.asset_class.clone(),
                    current_quantity: row.current_quantity.unwrap_or(0.0),
                    last_price: row.last_price,
                    notes: row.notes,
                });
            }
        }

        if !entries.is_empty() {
            self.portfolio.import_positions(&targets, &entries).await?;
        }
        tracing::info!(
            positions = targets.len(),
            ledger_rows = entries.len(),
            "Imported wallet positions"
        );
        Ok(targets.len())
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
        // key on every line, no notes column) used to insert one ledger row
        // per line.
        let (repo, service) = setup().await;
        let csv = "timestamp,symbol,group,barca,current_quantity\n\
                   t1,BTC,Core,Base,1.0\nt2,BTC,Core,Base,1.0\nt3,BTC,Core,Base,1.0\n";

        assert_eq!(service.import_csv(csv.as_bytes()).await.unwrap(), 1);
        assert_eq!(repo.count_ledger_rows("BTC").await, 1);
    }

    #[tokio::test]
    async fn each_source_of_a_position_becomes_its_own_ledger_row() {
        // Regression: only the last line of a multi-source position used to
        // be imported (USDT on Binance + GateIO kept only GateIO).
        let (repo, service) = setup().await;
        let csv = "symbol,group,barca,target_percent,current_quantity,comments,asset_class\n\
                   USDT,Core,Base,30,100,Binance,crypto\n\
                   USDT,Core,Base,0,50,GateIO,crypto\n\
                   USDT,Core,Base,0,7,,crypto\n";

        assert_eq!(service.import_csv(csv.as_bytes()).await.unwrap(), 1);

        let positions = repo.fetch_positions().await.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].current_quantity, 157.0);
        assert_eq!(positions[0].source_count, 3);
        assert_eq!(
            positions[0].target_percent, 30.0,
            "target is not lost to a later 0"
        );
    }

    #[tokio::test]
    async fn the_same_source_twice_keeps_its_last_line() {
        let (repo, service) = setup().await;
        let csv = format!(
            "{HEADER}BTC,Core,Base,10,1.0,Ledger,crypto\nBTC,Core,Base,10,2.0,Ledger,crypto\n"
        );

        service.import_csv(csv.as_bytes()).await.unwrap();

        assert_eq!(repo.count_ledger_rows("BTC").await, 1);
        assert_eq!(
            repo.fetch_positions().await.unwrap()[0].current_quantity,
            2.0
        );
    }

    #[tokio::test]
    async fn targets_summed_over_100_are_rejected() {
        let (repo, service) = setup().await;
        let csv = format!("{HEADER}BTC,Core,Base,60,1.0,A,crypto\nBTC,Core,Base,60,1.0,B,crypto\n");

        let err = service.import_csv(csv.as_bytes()).await.unwrap_err();

        assert!(matches!(err, ImportError::InvalidRows(_)), "{err:?}");
        assert!(repo.fetch_positions().await.unwrap().is_empty());
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
