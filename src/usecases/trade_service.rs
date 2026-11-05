//! Recording a buy or sell made on an exchange ("Record trade" on the
//! Rebalance page), so the wallet follows the trade without editing the
//! wallet CSV and importing it again.
//!
//! A position's quantity is the sum of its sources (one per exchange or
//! wallet, the ledger's `notes`), and each source's latest ledger row wins.
//! So a trade appends one row for the traded source with its old quantity
//! plus (buy) or minus (sell) the traded quantity — and, optionally, one
//! for the cash source that paid for it or received the proceeds. Both rows
//! and the `trades` log entry are written in one transaction.

use crate::domain::models::{
    LedgerEntry, PortfolioId, PositionKey, PositionSource, RecordedActivity, SourceChange,
    TradeRecord, TradeSide,
};
use crate::domain::repository::{PortfolioRepo, RepoError, StaleSource};
use crate::usecases::validation::{check_asset_class, check_not_blank};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// Float dust left by `held - sold` when selling everything (e.g.
/// 0.30000000000000004 - 0.3) counts as zero, not as an oversell.
const QUANTITY_EPSILON: f64 = 1e-9;

/// How many times a write is tried when another write changed one of its
/// sources in between (two tabs saving at once). Each try reads again.
const WRITE_ATTEMPTS: usize = 3;

#[derive(Debug)]
pub enum TradeError {
    /// The request itself is wrong (HTTP 400). The message says why.
    Invalid(String),
    /// The position to trade doesn't exist (HTTP 404).
    NotFound(String),
    /// Other writes kept changing the same source (HTTP 409).
    Conflict(String),
    Repo(RepoError),
}

impl fmt::Display for TradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradeError::Invalid(message)
            | TradeError::NotFound(message)
            | TradeError::Conflict(message) => {
                write!(f, "{message}")
            }
            TradeError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TradeError {}

impl From<RepoError> for TradeError {
    fn from(e: RepoError) -> Self {
        TradeError::Repo(e)
    }
}

impl From<String> for TradeError {
    fn from(message: String) -> Self {
        TradeError::Invalid(message)
    }
}

/// One source of one position: the position's key plus the exchange/wallet.
/// `source` may name an exchange the position isn't held on yet (a buy on a
/// new exchange); blank means the position's unnamed source.
#[derive(Debug, Clone, Deserialize)]
pub struct SourceRef {
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub asset_class: String,
    #[serde(default)]
    pub source: Option<String>,
}

impl SourceRef {
    /// The same reference with names trimmed, so " Base" and "Base" are one
    /// position everywhere (comparisons, lookups and new rows alike).
    fn trimmed(&self) -> SourceRef {
        let trim = |value: &Option<String>| value.as_deref().map(|v| v.trim().to_string());
        SourceRef {
            symbol: self.symbol.trim().to_string(),
            group_name: trim(&self.group_name),
            barca: trim(&self.barca),
            asset_class: self.asset_class.trim().to_string(),
            source: trim(&self.source),
        }
    }

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
        check_asset_class(&self.asset_class)
    }
}

/// The cash side of a trade: units of the cash asset (e.g. USDT in Caixa)
/// spent on a buy or received from a sell.
#[derive(Debug, Clone, Deserialize)]
pub struct CashLeg {
    pub position: SourceRef,
    pub quantity: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TradeRequest {
    pub side: TradeSide,
    pub asset: SourceRef,
    pub quantity: f64,
    /// Fill price in USD.
    pub price: f64,
    #[serde(default)]
    pub cash: Option<CashLeg>,
}

/// The new quantities of the sources the trade changed.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TradeOutcome {
    pub asset_quantity: f64,
    pub cash_quantity: Option<f64>,
}

pub struct TradeService {
    portfolio: Arc<dyn PortfolioRepo>,
}

impl TradeService {
    pub fn new(portfolio: Arc<dyn PortfolioRepo>) -> Self {
        Self { portfolio }
    }

    /// Every source of every position (one row per exchange/wallet), for
    /// the dialog's exchange picker.
    pub async fn list_sources(
        &self,
        portfolio: PortfolioId,
    ) -> Result<Vec<PositionSource>, TradeError> {
        Ok(self.portfolio.fetch_position_sources(portfolio).await?)
    }

    /// What has been recorded in the app, for the CSV import's warning.
    pub async fn recorded_activity(
        &self,
        portfolio: PortfolioId,
    ) -> Result<RecordedActivity, TradeError> {
        Ok(self.portfolio.fetch_recorded_activity(portfolio).await?)
    }

    pub async fn record(
        &self,
        portfolio: PortfolioId,
        request: TradeRequest,
        recorded_by: Option<i64>,
    ) -> Result<TradeOutcome, TradeError> {
        let request = TradeRequest {
            asset: request.asset.trimmed(),
            cash: request.cash.map(|cash| CashLeg {
                position: cash.position.trimmed(),
                ..cash
            }),
            ..request
        };
        request.asset.validate()?;
        check_positive("quantity", request.quantity)?;
        check_positive("price", request.price)?;
        if let Some(cash) = &request.cash {
            cash.position.validate()?;
            check_positive("cash quantity", cash.quantity)?;
            if cash.position.key() == request.asset.key() {
                return Err(TradeError::Invalid(
                    "the cash position must be a different position from the one traded".into(),
                ));
            }
        }

        for attempt in 1..=WRITE_ATTEMPTS {
            // Read on every attempt: a retry starts from the new quantities
            // (and is checked again for overselling).
            let sources = self.portfolio.fetch_position_sources(portfolio).await?;
            let (trade, changes) = plan_trade(&sources, &request, recorded_by)?;
            match self
                .portfolio
                .record_trade(portfolio, &trade, &changes)
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        portfolio_id = %portfolio,
                        side = trade.side.as_str(),
                        symbol = %trade.symbol,
                        source = trade.source.as_deref().unwrap_or(""),
                        quantity = trade.quantity,
                        price = trade.price,
                        cash_symbol = trade.cash_symbol.as_deref().unwrap_or(""),
                        attempt,
                        "Recorded trade"
                    );
                    return Ok(TradeOutcome {
                        asset_quantity: changes[0].entry.current_quantity,
                        cash_quantity: changes.get(1).map(|c| c.entry.current_quantity),
                    });
                }
                Err(e) if is_stale(&e) => {
                    tracing::warn!(portfolio_id = %portfolio, attempt, error = %e, "Trade source changed; retrying");
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(conflict())
    }
}

/// The trade's log row and its ledger changes (the traded source first,
/// then the cash source if any).
fn plan_trade(
    sources: &[PositionSource],
    request: &TradeRequest,
    recorded_by: Option<i64>,
) -> Result<(TradeRecord, Vec<SourceChange>), TradeError> {
    // Buying adds the asset and spends cash; selling does the opposite.
    let sign = match request.side {
        TradeSide::Buy => 1.0,
        TradeSide::Sell => -1.0,
    };
    let asset = next_entry(
        sources,
        &request.asset,
        sign * request.quantity,
        Some(request.price),
    )?;
    let cash = match &request.cash {
        Some(cash) => Some(next_entry(
            sources,
            &cash.position,
            -sign * cash.quantity,
            None,
        )?),
        None => None,
    };
    let trade = TradeRecord {
        side: request.side,
        symbol: asset.entry.symbol.clone(),
        group_name: asset.entry.group_name.clone(),
        barca: asset.entry.barca.clone(),
        asset_class: asset.entry.asset_class.clone(),
        source: asset.entry.notes.clone(),
        quantity: request.quantity,
        price: request.price,
        cash_symbol: cash.as_ref().map(|c| c.entry.symbol.clone()),
        cash_source: cash.as_ref().and_then(|c| c.entry.notes.clone()),
        cash_quantity: request.cash.as_ref().map(|c| c.quantity),
        recorded_by,
    };
    let mut changes = vec![asset];
    changes.extend(cash);
    Ok((trade, changes))
}

fn is_stale(e: &RepoError) -> bool {
    e.downcast_ref::<StaleSource>().is_some()
}

fn conflict() -> TradeError {
    TradeError::Conflict(
        "This balance kept changing while saving (another tab or import?). Check it and try again."
            .into(),
    )
}

fn check_positive(field: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{field} must be a number > 0, got {value}"));
    }
    Ok(())
}

/// Blank and missing both mean "the unnamed source", as in the
/// `wallet_allocations_current` view (`COALESCE(notes, '')`).
fn source_name(notes: Option<&str>) -> &str {
    notes.map(str::trim).unwrap_or("")
}

/// The ledger row that moves `target`'s source by `change` units, with the
/// quantity it started from. The position must exist; the source may be new
/// (it starts at 0). A source can't go below zero.
fn next_entry(
    sources: &[PositionSource],
    target: &SourceRef,
    change: f64,
    last_price: Option<f64>,
) -> Result<SourceChange, TradeError> {
    let key = target.key();
    let of_position: Vec<&PositionSource> = sources.iter().filter(|s| s.key() == key).collect();
    // Any source of the position gives the key exactly as stored (NULL vs
    // ''), so the new row lands in the same position.
    let template = of_position.first().ok_or_else(|| {
        TradeError::NotFound(format!(
            "no position for {} in {}",
            target.symbol,
            if key.barca.is_empty() {
                "this account"
            } else {
                &key.barca
            }
        ))
    })?;
    let wanted = source_name(target.source.as_deref());
    // "binance" is the existing "Binance", not a second exchange; an exact
    // match wins if both spellings are already stored.
    let existing = of_position
        .iter()
        .find(|s| source_name(s.notes.as_deref()) == wanted)
        .or_else(|| {
            of_position
                .iter()
                .find(|s| source_name(s.notes.as_deref()).eq_ignore_ascii_case(wanted))
        });
    let held = existing.map_or(0.0, |s| s.current_quantity);

    let mut quantity = held + change;
    if quantity < 0.0 && quantity > -QUANTITY_EPSILON {
        quantity = 0.0;
    }
    if quantity < 0.0 {
        let place = if wanted.is_empty() {
            "the unnamed source".to_string()
        } else {
            wanted.to_string()
        };
        return Err(TradeError::Invalid(format!(
            "{place} holds {held} {}, so it can't give {}",
            target.symbol, -change
        )));
    }

    let entry = LedgerEntry {
        symbol: template.symbol.clone(),
        group_name: template.group_name.clone(),
        barca: template.barca.clone(),
        asset_class: template.asset_class.clone(),
        current_quantity: quantity,
        last_price,
        // An existing source keeps its notes exactly, so the new row
        // replaces that source instead of starting a second one.
        notes: match existing {
            Some(s) => s.notes.clone(),
            None if wanted.is_empty() => None,
            None => Some(wanted.to_string()),
        },
    };
    Ok(SourceChange {
        entry,
        before: held,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::NewPortfolioTarget;
    use crate::infra::sqlite::{SqliteRepo, test_repo};

    const PORTFOLIO: PortfolioId = PortfolioId::FIRST;

    fn entry(symbol: &str, barca: &str, qty: f64, notes: Option<&str>) -> LedgerEntry {
        LedgerEntry {
            symbol: symbol.into(),
            group_name: Some(barca.into()),
            barca: Some(barca.into()),
            asset_class: "crypto".into(),
            current_quantity: qty,
            last_price: None,
            notes: notes.map(String::from),
        }
    }

    fn at(symbol: &str, barca: &str, source: Option<&str>) -> SourceRef {
        SourceRef {
            symbol: symbol.into(),
            group_name: Some(barca.into()),
            barca: Some(barca.into()),
            asset_class: "crypto".into(),
            source: source.map(String::from),
        }
    }

    /// BTC on Ledger (1.0) and Binance (0.5); USDT in Caixa on Binance
    /// (1000); SYRUP with an unnamed source (10).
    async fn service() -> (TradeService, Arc<SqliteRepo>) {
        let repo = test_repo().await;
        repo.import_positions(
            PORTFOLIO,
            &[] as &[NewPortfolioTarget],
            &[
                entry("BTC", "Base", 1.0, Some("Ledger")),
                entry("BTC", "Base", 0.5, Some("Binance")),
                entry("USDT", "Caixa", 1000.0, Some("Binance")),
                entry("SYRUP", "Altcoins", 10.0, None),
            ],
        )
        .await
        .unwrap();
        (TradeService::new(repo.clone()), repo)
    }

    async fn held(repo: &SqliteRepo, symbol: &str, source: &str) -> Option<f64> {
        repo.fetch_position_sources(PORTFOLIO)
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.symbol == symbol && source_name(s.notes.as_deref()) == source)
            .map(|s| s.current_quantity)
    }

    fn buy(asset: SourceRef, quantity: f64, price: f64) -> TradeRequest {
        TradeRequest {
            side: TradeSide::Buy,
            asset,
            quantity,
            price,
            cash: None,
        }
    }

    #[tokio::test]
    async fn a_buy_adds_to_that_exchange_only_and_spends_the_cash() {
        let (service, repo) = service().await;
        let request = TradeRequest {
            cash: Some(CashLeg {
                position: at("USDT", "Caixa", Some("Binance")),
                quantity: 400.0,
            }),
            ..buy(at("BTC", "Base", Some("Binance")), 0.01, 40_000.0)
        };
        let outcome = service.record(PORTFOLIO, request, None).await.unwrap();

        assert_eq!(
            outcome,
            TradeOutcome {
                asset_quantity: 0.51,
                cash_quantity: Some(600.0)
            }
        );
        assert_eq!(held(&repo, "BTC", "Binance").await, Some(0.51));
        assert_eq!(held(&repo, "BTC", "Ledger").await, Some(1.0));
        assert_eq!(held(&repo, "USDT", "Binance").await, Some(600.0));
    }

    #[tokio::test]
    async fn a_sell_takes_from_the_exchange_and_the_proceeds_go_to_cash() {
        let (service, repo) = service().await;
        let request = TradeRequest {
            side: TradeSide::Sell,
            cash: Some(CashLeg {
                position: at("USDT", "Caixa", Some("Binance")),
                quantity: 20_000.0,
            }),
            ..buy(at("BTC", "Base", Some("Binance")), 0.5, 40_000.0)
        };
        service.record(PORTFOLIO, request, None).await.unwrap();

        assert_eq!(held(&repo, "BTC", "Binance").await, Some(0.0));
        assert_eq!(held(&repo, "USDT", "Binance").await, Some(21_000.0));
    }

    #[tokio::test]
    async fn selling_everything_is_not_an_oversell_because_of_float_dust() {
        let (service, repo) = service().await;
        service
            .record(
                PORTFOLIO,
                buy(at("BTC", "Base", Some("Binance")), 0.1, 1.0),
                None,
            )
            .await
            .unwrap();
        // 0.6 - 0.6 in floats can come out a hair below zero.
        let request = TradeRequest {
            side: TradeSide::Sell,
            ..buy(at("BTC", "Base", Some("Binance")), 0.6, 1.0)
        };
        service.record(PORTFOLIO, request, None).await.unwrap();
        assert_eq!(held(&repo, "BTC", "Binance").await, Some(0.0));
    }

    #[tokio::test]
    async fn a_buy_on_a_new_exchange_starts_a_new_source() {
        let (service, repo) = service().await;
        service
            .record(
                PORTFOLIO,
                buy(at("BTC", "Base", Some("  Kraken ")), 0.2, 40_000.0),
                None,
            )
            .await
            .unwrap();
        assert_eq!(held(&repo, "BTC", "Kraken").await, Some(0.2));
        let positions = repo.fetch_positions(PORTFOLIO).await.unwrap();
        let btc = positions.iter().find(|p| p.symbol == "BTC").unwrap();
        assert!((btc.current_quantity - 1.7).abs() < 1e-12);
        assert_eq!(btc.source_count, 3);
    }

    #[tokio::test]
    async fn names_are_matched_trimmed_and_the_exchange_ignoring_case() {
        let (service, repo) = service().await;
        let mut asset = at(" BTC ", "Base ", Some(" binance "));
        asset.group_name = Some(" Base".into());
        service
            .record(PORTFOLIO, buy(asset, 0.1, 40_000.0), None)
            .await
            .unwrap();
        assert_eq!(held(&repo, "BTC", "Binance").await, Some(0.6));
        let positions = repo.fetch_positions(PORTFOLIO).await.unwrap();
        let btc: Vec<_> = positions.iter().filter(|p| p.symbol == "BTC").collect();
        assert_eq!(btc.len(), 1, "no second BTC position");
        assert_eq!(btc[0].source_count, 2, "no second Binance source");
    }

    /// A real file database (WAL, several connections), like production:
    /// an in-memory one can't run two writes at the same time.
    async fn file_service(name: &str) -> (Arc<TradeService>, Arc<SqliteRepo>, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "trade-{name}-{}-{}.db",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let pool = crate::infra::sqlite::connect(&format!("sqlite://{}", path.display()))
            .await
            .unwrap();
        let repo = Arc::new(SqliteRepo::new(pool));
        repo.import_positions(
            PORTFOLIO,
            &[] as &[NewPortfolioTarget],
            &[entry("BTC", "Base", 0.5, Some("Binance"))],
        )
        .await
        .unwrap();
        (Arc::new(TradeService::new(repo.clone())), repo, path)
    }

    async fn trade_count(repo: &SqliteRepo) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM trades")
            .fetch_one(repo.pool())
            .await
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn trades_saved_at_the_same_time_are_all_counted() {
        let (service, repo, path) = file_service("buys").await;
        let tasks: Vec<_> = (0..20)
            .map(|_| {
                let service = service.clone();
                tokio::spawn(async move {
                    service
                        .record(
                            PORTFOLIO,
                            buy(at("BTC", "Base", Some("Binance")), 0.1, 1.0),
                            None,
                        )
                        .await
                })
            })
            .collect();
        let mut recorded = 0;
        for task in tasks {
            match task.await.unwrap() {
                Ok(_) => recorded += 1,
                // Allowed (the user is asked to try again), but never a
                // silent loss.
                Err(TradeError::Conflict(_)) => {}
                Err(e) => panic!("unexpected error: {e}"),
            }
        }
        assert!(recorded > 0);
        let now = held(&repo, "BTC", "Binance").await.unwrap();
        assert!(
            (now - (0.5 + 0.1 * recorded as f64)).abs() < 1e-9,
            "{recorded} buys recorded but Binance holds {now}"
        );
        assert_eq!(trade_count(&repo).await, recorded);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn two_sells_at_the_same_time_cant_oversell() {
        let (service, repo, path) = file_service("sells").await;
        let sell = || TradeRequest {
            side: TradeSide::Sell,
            ..buy(at("BTC", "Base", Some("Binance")), 0.4, 1.0)
        };
        let (a, b) = tokio::join!(
            tokio::spawn({
                let service = service.clone();
                async move { service.record(PORTFOLIO, sell(), None).await }
            }),
            tokio::spawn({
                let service = service.clone();
                async move { service.record(PORTFOLIO, sell(), None).await }
            })
        );
        let results = [a.unwrap(), b.unwrap()];
        assert_eq!(
            results.iter().filter(|r| r.is_ok()).count(),
            1,
            "{results:?}"
        );
        assert!((held(&repo, "BTC", "Binance").await.unwrap() - 0.1).abs() < 1e-9);
        assert_eq!(trade_count(&repo).await, 1);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn a_blank_exchange_means_the_unnamed_source() {
        let (service, repo) = service().await;
        let outcome = service
            .record(
                PORTFOLIO,
                buy(at("SYRUP", "Altcoins", Some("")), 5.0, 0.5),
                None,
            )
            .await
            .unwrap();
        assert_eq!(outcome.asset_quantity, 15.0);
        assert_eq!(held(&repo, "SYRUP", "").await, Some(15.0));
        let positions = repo.fetch_positions(PORTFOLIO).await.unwrap();
        let syrup = positions.iter().find(|p| p.symbol == "SYRUP").unwrap();
        assert_eq!(syrup.source_count, 1);
    }

    #[tokio::test]
    async fn the_trade_is_logged_with_its_price() {
        let (service, repo) = service().await;
        service
            .record(
                PORTFOLIO,
                buy(at("SYRUP", "Altcoins", None), 5.0, 0.5),
                None,
            )
            .await
            .unwrap();
        let (side, symbol, price): (String, String, f64) =
            sqlx::query_as("SELECT side, symbol, price FROM trades")
                .fetch_one(repo.pool())
                .await
                .unwrap();
        assert_eq!(
            (side.as_str(), symbol.as_str(), price),
            ("buy", "SYRUP", 0.5)
        );
    }

    #[tokio::test]
    async fn overselling_or_overspending_is_refused_and_writes_nothing() {
        let (service, repo) = service().await;
        let oversell = TradeRequest {
            side: TradeSide::Sell,
            ..buy(at("BTC", "Base", Some("Binance")), 0.6, 40_000.0)
        };
        let err = service.record(PORTFOLIO, oversell, None).await.unwrap_err();
        assert!(
            matches!(&err, TradeError::Invalid(m) if m.contains("Binance holds 0.5 BTC")),
            "{err}"
        );

        // The buy itself is fine, but the cash can't cover it: neither row
        // is written.
        let overspend = TradeRequest {
            cash: Some(CashLeg {
                position: at("USDT", "Caixa", Some("Binance")),
                quantity: 5000.0,
            }),
            ..buy(at("BTC", "Base", Some("Binance")), 0.1, 50_000.0)
        };
        assert!(matches!(
            service.record(PORTFOLIO, overspend, None).await,
            Err(TradeError::Invalid(_))
        ));
        assert_eq!(held(&repo, "BTC", "Binance").await, Some(0.5));
        assert_eq!(held(&repo, "USDT", "Binance").await, Some(1000.0));
    }

    #[tokio::test]
    async fn bad_numbers_and_unknown_positions_are_refused() {
        let (service, _) = service().await;
        for (quantity, price) in [(0.0, 1.0), (-1.0, 1.0), (1.0, 0.0), (f64::NAN, 1.0)] {
            let request = buy(at("BTC", "Base", Some("Binance")), quantity, price);
            assert!(
                matches!(
                    service.record(PORTFOLIO, request, None).await,
                    Err(TradeError::Invalid(_))
                ),
                "quantity {quantity}, price {price}"
            );
        }
        let ghost = buy(at("GHOST", "Base", Some("Binance")), 1.0, 1.0);
        assert!(matches!(
            service.record(PORTFOLIO, ghost, None).await,
            Err(TradeError::NotFound(_))
        ));
        // BTC exists, but not in the Altcoins BARCA.
        let wrong_barca = buy(at("BTC", "Altcoins", Some("Binance")), 1.0, 1.0);
        assert!(matches!(
            service.record(PORTFOLIO, wrong_barca, None).await,
            Err(TradeError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn the_cash_must_be_another_position() {
        let (service, _) = service().await;
        let request = TradeRequest {
            cash: Some(CashLeg {
                position: at("BTC", "Base", Some("Ledger")),
                quantity: 0.1,
            }),
            ..buy(at("BTC", "Base", Some("Binance")), 0.1, 1.0)
        };
        assert!(matches!(
            service.record(PORTFOLIO, request, None).await,
            Err(TradeError::Invalid(_))
        ));
    }
}
