//! Imports a wallet CSV (upload, startup bootstrap, or the
//! `import_wallet_allocations` binary).
//!
//! The CSV wins for quantities, the database wins for targets:
//!
//! - A new position (symbol, group, barca, asset_class) is added with its
//!   quantities and its CSV target.
//! - An existing position keeps its database target. Each of its sources
//!   whose CSV quantity differs (or that is new) gets a new ledger row;
//!   unchanged sources are not touched, so re-importing a file writes
//!   nothing. A CSV target that differs from the database is only reported.
//! - A database source the CSV doesn't mention is left as is and reported,
//!   so a partial CSV never zeroes the rest of the wallet.
//!
//! A position may span several lines, one per funding source (the `notes`
//! / `comments` column: "Binance", "Ledger Wallet"...). Each source becomes
//! its own ledger row, and the position's target is the sum of its lines'
//! targets (the wallet CSV puts it on one line and 0 on the others).
//!
//! Sources are compared one by one with `fetch_position_sources`, never
//! with the summed `fetch_positions` quantity: writing a sum back as one
//! source would double it.

use crate::domain::models::{
    LedgerEntry, NewPortfolioTarget, PositionKey, PositionSource, WalletPosition,
};
use crate::domain::repository::{PortfolioRepo, RepoError};
use crate::usecases::validation::{
    check_asset_class, check_not_blank, check_percent, check_quantity,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
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
    /// The sum of its lines' targets; `None` when no line has one (e.g. the
    /// file has no `target_percent` column).
    target_percent: Option<f64>,
    sources: Vec<WalletCsvRow>,
}

/// Identifies one funding source: a position plus its `notes`. No notes and
/// empty notes are the same source, like `COALESCE(notes, '')` in the
/// `wallet_allocations_current` view.
type SourceKey = (PositionKey, String);

fn source_key(key: PositionKey, notes: &Option<String>) -> SourceKey {
    (key, notes.clone().unwrap_or_default())
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

    let mut sources: BTreeMap<SourceKey, WalletCsvRow> = BTreeMap::new();
    let mut errors = Vec::new();
    for record in csv_reader.records() {
        let record = record?;
        let line = record.position().map_or(0, |p| p.line());
        let row: WalletCsvRow = record.deserialize(Some(&headers))?;
        match row.validate() {
            Ok(()) => {
                sources.insert(source_key(row.key(), &row.notes), row);
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
            target_percent: None,
            sources: Vec::new(),
        });
        if let Some(percent) = row.target_percent {
            position.target_percent = Some(position.target_percent.unwrap_or(0.0) + percent);
        }
        position.sources.push(row);
    }
    let errors: Vec<String> = positions
        .iter()
        .filter_map(|(key, position)| {
            check_percent(position.target_percent.unwrap_or(0.0))
                .err()
                .map(|message| {
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

/// What an import changed, and what it found but deliberately left alone.
/// Returned to the "Import CSV" button and printed by the binary.
#[derive(Debug, Default, Serialize, PartialEq)]
pub struct ImportSummary {
    /// Positions that didn't exist yet (added with their CSV target).
    pub positions_added: usize,
    /// Sources not in the database yet, including those of new positions.
    pub sources_added: usize,
    /// Existing sources whose quantity changed.
    pub sources_updated: usize,
    /// Existing sources already at the CSV quantity (nothing written).
    pub sources_unchanged: usize,
    /// Database sources with a quantity that the CSV doesn't mention.
    /// Left as is: the CSV may be partial.
    pub missing_from_csv: Vec<MissingSource>,
    /// Existing positions whose CSV target differs from the database. The
    /// database target is kept; change it in the Targets tab if needed.
    pub target_conflicts: Vec<TargetConflict>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MissingSource {
    #[serde(flatten)]
    pub key: PositionKey,
    pub notes: Option<String>,
    pub current_quantity: f64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TargetConflict {
    #[serde(flatten)]
    pub key: PositionKey,
    pub database_percent: f64,
    pub csv_percent: f64,
}

/// The writes an import needs, plus its summary.
struct ImportPlan {
    targets: Vec<NewPortfolioTarget>,
    entries: Vec<LedgerEntry>,
    summary: ImportSummary,
}

/// Compares the CSV with the database and decides what to write (see the
/// module docs for the rules). Pure: no I/O, so every rule is unit tested.
fn plan_import(
    csv: BTreeMap<PositionKey, CsvPosition>,
    db_positions: &[WalletPosition],
    db_sources: &[PositionSource],
) -> ImportPlan {
    let db_targets: HashMap<PositionKey, f64> = db_positions
        .iter()
        .map(|p| (p.key(), p.target_percent))
        .collect();
    let db_quantities: HashMap<SourceKey, f64> = db_sources
        .iter()
        .map(|s| (source_key(s.key(), &s.notes), s.current_quantity))
        .collect();

    let mut plan = ImportPlan {
        targets: Vec::new(),
        entries: Vec::new(),
        summary: ImportSummary::default(),
    };
    let mut sources_in_csv = HashSet::new();

    for (key, position) in csv {
        match db_targets.get(&key) {
            None => {
                plan.summary.positions_added += 1;
                plan.targets.push(NewPortfolioTarget {
                    key: key.clone(),
                    target_percent: position.target_percent.unwrap_or(0.0),
                });
            }
            Some(&database_percent) => match position.target_percent {
                Some(csv_percent) if csv_percent != database_percent => {
                    plan.summary.target_conflicts.push(TargetConflict {
                        key: key.clone(),
                        database_percent,
                        csv_percent,
                    });
                }
                _ => {}
            },
        }

        for row in position.sources {
            let source = source_key(key.clone(), &row.notes);
            let old_quantity = db_quantities.get(&source).copied();
            sources_in_csv.insert(source);

            let new_quantity = match (old_quantity, row.current_quantity) {
                (None, quantity) => {
                    plan.summary.sources_added += 1;
                    quantity.unwrap_or(0.0)
                }
                // A blank quantity says nothing about an existing source;
                // it must not zero it.
                (Some(_), None) => {
                    plan.summary.sources_unchanged += 1;
                    continue;
                }
                // Exact comparison is fine: both sides are the same f64
                // parsed from the same text, stored as a SQLite REAL.
                (Some(old), Some(new)) if old == new => {
                    plan.summary.sources_unchanged += 1;
                    continue;
                }
                (Some(old), Some(new)) => {
                    tracing::debug!(
                        symbol = %key.symbol,
                        group = %key.group_name,
                        source = row.notes.as_deref().unwrap_or(""),
                        old,
                        new,
                        "Wallet import updates a source quantity"
                    );
                    plan.summary.sources_updated += 1;
                    new
                }
            };
            plan.entries.push(LedgerEntry {
                symbol: key.symbol.clone(),
                group_name: row.group,
                barca: row.barca,
                asset_class: key.asset_class.clone(),
                current_quantity: new_quantity,
                last_price: row.last_price,
                notes: row.notes,
            });
        }
    }

    plan.summary.missing_from_csv = db_sources
        .iter()
        .filter(|s| s.current_quantity != 0.0)
        .filter(|s| !sources_in_csv.contains(&source_key(s.key(), &s.notes)))
        .map(|s| MissingSource {
            key: s.key(),
            notes: s.notes.clone(),
            current_quantity: s.current_quantity,
        })
        .collect();
    plan
}

pub struct WalletImportService {
    portfolio: Arc<dyn PortfolioRepo>,
}

impl WalletImportService {
    pub fn new(portfolio: Arc<dyn PortfolioRepo>) -> Self {
        Self { portfolio }
    }

    /// Imports `reader` (see the module docs for the rules). All-or-nothing:
    /// a bad row means nothing is written, and every write is one
    /// transaction.
    pub async fn import_csv(&self, reader: impl Read) -> Result<ImportSummary, ImportError> {
        let csv = parse_rows(reader)?;
        let positions = self.portfolio.fetch_positions().await?;
        let sources = self.portfolio.fetch_position_sources().await?;

        let plan = plan_import(csv, &positions, &sources);
        if !plan.entries.is_empty() {
            self.portfolio
                .import_positions(&plan.targets, &plan.entries)
                .await?;
        }

        let summary = plan.summary;
        tracing::info!(
            positions_added = summary.positions_added,
            sources_added = summary.sources_added,
            sources_updated = summary.sources_updated,
            sources_unchanged = summary.sources_unchanged,
            missing_from_csv = summary.missing_from_csv.len(),
            target_conflicts = summary.target_conflicts.len(),
            "Imported wallet CSV"
        );
        Ok(summary)
    }

    pub async fn import_csv_file(&self, path: &str) -> Result<ImportSummary, ImportError> {
        let file = std::fs::File::open(path).map_err(|e| ImportError::Csv(e.into()))?;
        self.import_csv(file).await
    }

    /// On first start (empty portfolio), seeds from the CSV at `path`.
    /// Failures are logged, not fatal: the app works without a seed file.
    pub async fn seed_if_empty(&self, path: &str) {
        match self.portfolio.fetch_positions().await {
            Ok(positions) if positions.is_empty() => match self.import_csv_file(path).await {
                Ok(summary) => tracing::info!(
                    path,
                    imported = summary.positions_added,
                    "Seeded portfolio from CSV"
                ),
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

    fn key(symbol: &str) -> PositionKey {
        target(symbol, 0.0).key
    }

    const HEADER: &str = "symbol,group,barca,target_percent,current_quantity,notes,asset_class\n";

    #[tokio::test]
    async fn imports_every_row_into_an_empty_portfolio() {
        let (repo, service) = setup().await;
        let csv =
            format!("{HEADER}BTC,Core,Base,50,1.0,seed,crypto\nETH,Core,Base,50,2.0,seed,crypto\n");

        assert_eq!(
            service
                .import_csv(csv.as_bytes())
                .await
                .unwrap()
                .positions_added,
            2
        );
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

        assert_eq!(
            service
                .import_csv(csv.as_bytes())
                .await
                .unwrap()
                .positions_added,
            1
        );
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

        assert_eq!(
            service
                .import_csv(csv.as_bytes())
                .await
                .unwrap()
                .positions_added,
            1
        );

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
    async fn an_existing_position_takes_the_csv_quantity_but_keeps_its_target() {
        let (repo, service) = setup().await;
        repo.import_positions(&[target("BTC", 40.0)], &[entry("BTC", 1.0, Some("seed"))])
            .await
            .unwrap();

        let csv = format!("{HEADER}BTC,Core,Base,99,5.0,seed,crypto\n");
        let summary = service.import_csv(csv.as_bytes()).await.unwrap();

        assert_eq!(summary.positions_added, 0);
        assert_eq!(summary.sources_updated, 1);
        assert_eq!(repo.target_of("BTC").await, Some(40.0));
        let position = &repo.fetch_positions().await.unwrap()[0];
        assert_eq!(position.current_quantity, 5.0);
        assert_eq!(
            position.source_count, 1,
            "updated, not added as a new source"
        );
        assert_eq!(
            summary.target_conflicts,
            [TargetConflict {
                key: key("BTC"),
                database_percent: 40.0,
                csv_percent: 99.0,
            }]
        );
    }

    #[tokio::test]
    async fn a_new_source_is_added_to_an_existing_position() {
        // Regression: LINK had Ledger + Coinbase in the database; the GateIO
        // line of the CSV was dropped because the position already existed.
        let (repo, service) = setup().await;
        repo.import_positions(
            &[target("LINK", 1.0)],
            &[
                entry("LINK", 121.0, Some("Ledger")),
                entry("LINK", 22.0, Some("Coinbase")),
            ],
        )
        .await
        .unwrap();

        let csv = format!(
            "{HEADER}LINK,Core,Base,1,121.0,Ledger,crypto\n\
             LINK,Core,Base,0,22.0,Coinbase,crypto\n\
             LINK,Core,Base,0,18.0,GateIO,crypto\n"
        );
        let summary = service.import_csv(csv.as_bytes()).await.unwrap();

        assert_eq!(summary.sources_added, 1);
        assert_eq!(summary.sources_unchanged, 2);
        assert_eq!(
            repo.count_ledger_rows("LINK").await,
            3,
            "only GateIO written"
        );
        let position = &repo.fetch_positions().await.unwrap()[0];
        assert_eq!(position.current_quantity, 161.0);
        assert_eq!(position.source_count, 3);
    }

    #[tokio::test]
    async fn importing_the_same_file_twice_writes_nothing_the_second_time() {
        let (repo, service) = setup().await;
        let csv =
            format!("{HEADER}BTC,Core,Base,50,0.1,Ledger,crypto\nBTC,Core,Base,0,0.2,,crypto\n");
        service.import_csv(csv.as_bytes()).await.unwrap();

        let summary = service.import_csv(csv.as_bytes()).await.unwrap();

        assert_eq!(
            summary,
            ImportSummary {
                sources_unchanged: 2,
                ..ImportSummary::default()
            }
        );
        assert_eq!(repo.count_ledger_rows("BTC").await, 2);
    }

    #[tokio::test]
    async fn a_source_missing_from_the_csv_is_reported_not_zeroed() {
        // The CSV may be partial: only what it mentions changes.
        let (repo, service) = setup().await;
        repo.import_positions(
            &[target("BTC", 50.0), target("ETH", 50.0)],
            &[
                entry("BTC", 1.0, Some("Ledger")),
                entry("BTC", 2.0, Some("Binance")),
                entry("ETH", 3.0, None),
                entry("SOL", 0.0, None),
            ],
        )
        .await
        .unwrap();

        let csv = format!("{HEADER}BTC,Core,Base,50,1.0,Ledger,crypto\n");
        let summary = service.import_csv(csv.as_bytes()).await.unwrap();

        let missing: Vec<_> = summary
            .missing_from_csv
            .iter()
            .map(|m| {
                (
                    m.key.symbol.as_str(),
                    m.notes.as_deref(),
                    m.current_quantity,
                )
            })
            .collect();
        assert_eq!(
            missing,
            [("BTC", Some("Binance"), 2.0), ("ETH", None, 3.0)],
            "a source already at 0 (SOL) is not worth reporting"
        );
        let quantities: Vec<_> = repo
            .fetch_positions()
            .await
            .unwrap()
            .iter()
            .map(|p| p.current_quantity)
            .collect();
        assert_eq!(quantities, [3.0, 3.0, 0.0], "nothing was zeroed");
    }

    #[tokio::test]
    async fn a_blank_quantity_leaves_an_existing_source_alone() {
        let (repo, service) = setup().await;
        repo.import_positions(&[target("BTC", 50.0)], &[entry("BTC", 1.0, Some("Ledger"))])
            .await
            .unwrap();

        let csv = "symbol,group,barca,notes\nBTC,Core,Base,Ledger\n";
        let summary = service.import_csv(csv.as_bytes()).await.unwrap();

        assert_eq!(summary.sources_unchanged, 1);
        assert!(
            summary.target_conflicts.is_empty(),
            "no target column, no conflict"
        );
        assert_eq!(
            repo.fetch_positions().await.unwrap()[0].current_quantity,
            1.0
        );
    }

    #[tokio::test]
    async fn empty_and_missing_notes_are_the_same_source() {
        let (repo, service) = setup().await;
        repo.import_positions(&[target("BTC", 50.0)], &[entry("BTC", 1.0, None)])
            .await
            .unwrap();

        let csv = format!("{HEADER}BTC,Core,Base,50,2.0,,crypto\n");
        let summary = service.import_csv(csv.as_bytes()).await.unwrap();

        assert_eq!(summary.sources_updated, 1);
        assert!(summary.missing_from_csv.is_empty());
        assert_eq!(
            repo.fetch_positions().await.unwrap()[0].current_quantity,
            2.0
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
        assert_eq!(
            service
                .import_csv(csv.as_bytes())
                .await
                .unwrap()
                .positions_added,
            1
        );
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
