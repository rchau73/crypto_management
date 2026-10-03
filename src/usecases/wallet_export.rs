//! Exports the wallet as a CSV in the same format the importer reads (the
//! `wallet_allocations.csv` layout), so the current quantities can be used
//! in reports outside the app or imported into another database.
//!
//! One line per funding source, like the original CSV. A position's target
//! goes on its first line and 0 on the others, so the importer's "sum the
//! lines" rule gives back the same target. Two extra columns, `price_usd`
//! and `value_usd`, come from the latest "Update Prices" snapshot; the
//! importer ignores them.

use crate::domain::models::{PositionKey, PositionSource};
use crate::domain::repository::{PortfolioRepo, RepoResult, SnapshotRepo};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const HEADER: [&str; 9] = [
    "symbol",
    "group",
    "barca",
    "target_percent",
    "current_quantity",
    "comments",
    "asset_class",
    "price_usd",
    "value_usd",
];

pub struct WalletExportService {
    portfolio: Arc<dyn PortfolioRepo>,
    snapshots: Arc<dyn SnapshotRepo>,
}

impl WalletExportService {
    pub fn new(portfolio: Arc<dyn PortfolioRepo>, snapshots: Arc<dyn SnapshotRepo>) -> Self {
        Self {
            portfolio,
            snapshots,
        }
    }

    /// The whole wallet as CSV text.
    pub async fn export_csv(&self) -> RepoResult<String> {
        let sources = self.portfolio.fetch_position_sources().await?;
        let targets: HashMap<PositionKey, f64> = self
            .portfolio
            .fetch_positions()
            .await?
            .into_iter()
            .map(|p| (p.key(), p.target_percent))
            .collect();
        let prices = self.snapshots.fetch_latest_prices().await?;

        let csv = write_wallet_csv(&sources, &targets, &prices)?;
        tracing::info!(
            rows = sources.len(),
            priced_symbols = prices.len(),
            bytes = csv.len(),
            "Exported wallet CSV"
        );
        Ok(csv)
    }
}

fn source_key(source: &PositionSource) -> PositionKey {
    PositionKey {
        symbol: source.symbol.clone(),
        group_name: source.group_name.clone().unwrap_or_default(),
        barca: source.barca.clone().unwrap_or_default(),
        asset_class: source.asset_class.clone(),
    }
}

/// Builds the CSV. `sources` must be sorted by position (the repo does
/// this) so each position's target lands on its first line.
fn write_wallet_csv(
    sources: &[PositionSource],
    targets: &HashMap<PositionKey, f64>,
    prices: &HashMap<String, f64>,
) -> RepoResult<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(HEADER)?;

    let mut target_written: HashSet<PositionKey> = HashSet::new();
    for source in sources {
        let key = source_key(source);
        let target = if target_written.insert(key.clone()) {
            targets.get(&key).copied().unwrap_or(0.0)
        } else {
            0.0
        };
        let price = prices.get(&source.symbol).copied();

        // `{}` (Display) never uses exponent notation, so a dust balance
        // reads 0.0000003, not 3e-7, in a spreadsheet.
        writer.write_record([
            source.symbol.clone(),
            key.group_name,
            key.barca,
            target.to_string(),
            source.current_quantity.to_string(),
            source.notes.clone().unwrap_or_default(),
            source.asset_class.clone(),
            price.map(|p| p.to_string()).unwrap_or_default(),
            price
                .map(|p| format!("{:.2}", p * source.current_quantity))
                .unwrap_or_default(),
        ])?;
    }
    let bytes = writer.into_inner().map_err(|e| e.into_error())?;
    Ok(String::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{AllocationReport, AllocationSnapshot, AssetAllocation};
    use crate::infra::sqlite::test_repo;
    use crate::usecases::wallet_import::WalletImportService;

    fn source(symbol: &str, quantity: f64, notes: Option<&str>) -> PositionSource {
        PositionSource {
            symbol: symbol.into(),
            group_name: Some("Holding".into()),
            barca: Some("Caixa".into()),
            asset_class: "crypto".into(),
            current_quantity: quantity,
            notes: notes.map(str::to_string),
        }
    }

    fn key(symbol: &str) -> PositionKey {
        source_key(&source(symbol, 0.0, None))
    }

    #[test]
    fn writes_one_line_per_source_with_the_target_on_the_first() {
        let csv = write_wallet_csv(
            &[
                source("USDT", 100.0, Some("Binance")),
                source("USDT", 50.5, Some("GateIO")),
            ],
            &HashMap::from([(key("USDT"), 30.0)]),
            &HashMap::from([("USDT".to_string(), 1.0)]),
        )
        .unwrap();

        assert_eq!(
            csv,
            "symbol,group,barca,target_percent,current_quantity,comments,asset_class,price_usd,value_usd\n\
             USDT,Holding,Caixa,30,100,Binance,crypto,1,100.00\n\
             USDT,Holding,Caixa,0,50.5,GateIO,crypto,1,50.50\n"
        );
    }

    #[test]
    fn unpriced_symbols_and_missing_notes_leave_cells_empty() {
        let csv = write_wallet_csv(
            &[source("NEW", 2.0, None)],
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        assert!(csv.ends_with("NEW,Holding,Caixa,0,2,,crypto,,\n"), "{csv}");
    }

    #[test]
    fn notes_with_commas_or_quotes_are_escaped_and_tiny_quantities_avoid_exponents() {
        let csv = write_wallet_csv(
            &[source("ETH", 0.0000003, Some(r#"Metamask, "Arbitrum""#))],
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        assert!(
            csv.contains(r#"ETH,Holding,Caixa,0,0.0000003,"Metamask, ""Arbitrum""",crypto"#),
            "{csv}"
        );
    }

    #[test]
    fn an_empty_wallet_is_just_the_header() {
        let csv = write_wallet_csv(&[], &HashMap::new(), &HashMap::new()).unwrap();
        assert_eq!(csv.lines().count(), 1);
    }

    #[tokio::test]
    async fn export_then_import_into_an_empty_database_gives_the_same_positions() {
        let original = test_repo().await;
        let csv = "symbol,group,barca,target_percent,current_quantity,comments,asset_class\n\
                   USDT,Holding,Caixa,30,9696.35534618,Binance,crypto\n\
                   USDT,Holding,Caixa,0,707.5683419,Binance USDC,crypto\n\
                   BTC,Holding,Base,0,0.01733204,Binance,crypto\n\
                   BTC,Holding,Base,58,0.5494,Ledger Wallet,crypto\n\
                   XPML11,FII,RendaPassiva,12,10,,br-equities\n";
        WalletImportService::new(original.clone())
            .import_csv(csv.as_bytes())
            .await
            .unwrap();
        original
            .record_snapshot(&AllocationSnapshot {
                timestamp: "2026-10-02T00:00:00+00:00".into(),
                report: AllocationReport {
                    per_asset: vec![AssetAllocation {
                        symbol: "BTC".into(),
                        group: "Holding".into(),
                        barca: "Base".into(),
                        price: 100_000.0,
                        current_quantity: 0.0,
                        value: 0.0,
                        target_percent: 0.0,
                        current_percent: 0.0,
                        deviation: 0.0,
                    }],
                    ..Default::default()
                },
                asset_notes: HashMap::new(),
            })
            .await
            .unwrap();

        let exported = WalletExportService::new(original.clone(), original.clone())
            .export_csv()
            .await
            .unwrap();
        assert!(
            exported.contains("BTC,Holding,Base,0,0.5494,Ledger Wallet,crypto,100000,54940.00")
        );

        let copy = test_repo().await;
        WalletImportService::new(copy.clone())
            .import_csv(exported.as_bytes())
            .await
            .unwrap();
        assert_eq!(
            copy.fetch_positions().await.unwrap(),
            original.fetch_positions().await.unwrap()
        );
    }
}
