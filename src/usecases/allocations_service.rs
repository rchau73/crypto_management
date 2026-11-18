//! Live allocations: one set of prices shared by every portfolio, refreshed
//! at most once per `refresh_interval`, and each portfolio's report
//! computed from it on request.
//!
//! The app asks for the report every few seconds, but that only reads the
//! database: the price APIs (CoinMarketCap bills per call) are called when
//! the shared prices are older than the interval, or when a portfolio holds
//! a symbol they don't cover yet (a fresh import). Each refresh records one
//! history snapshot per portfolio, so the history grows with refreshes,
//! not with page views.

use crate::domain::market_data::{CryptoProvider, EquityProvider, FxProvider};
use crate::domain::models::{
    AllocationReport, AllocationSnapshot, AssetClass, FxRate, MarketQuote, PortfolioId,
    PricedReport, WalletPosition,
};
use crate::domain::repository::{
    BarcaTargetRepo, PortfolioDirectoryRepo, PortfolioRepo, RepoError, SnapshotRepo,
};
use crate::usecases::compute_allocations::compute_allocations;
use chrono::{DateTime, Utc};
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// After a failed refresh, wait this long before calling the APIs again,
/// so an outage isn't hit by every poll of the app.
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum AllocationsError {
    /// CoinMarketCap failed and there are no earlier prices to fall back
    /// on. Crypto is the core of the portfolio, so without it there is
    /// nothing meaningful to show.
    CryptoPrices(RepoError),
    Repo(RepoError),
}

impl fmt::Display for AllocationsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocationsError::CryptoPrices(e) => write!(f, "failed to fetch crypto prices: {e}"),
            AllocationsError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AllocationsError {}

impl From<RepoError> for AllocationsError {
    fn from(e: RepoError) -> Self {
        AllocationsError::Repo(e)
    }
}

/// The price sources, one per asset class, plus the USD/BRL rate that
/// turns the (BRL) Brazilian quotes into dollars.
pub struct MarketProviders {
    pub crypto: Arc<dyn CryptoProvider>,
    pub br_equities: Arc<dyn EquityProvider>,
    pub us_indices: Arc<dyn EquityProvider>,
    pub usd_brl: Arc<dyn FxProvider>,
}

/// Every quote in USD, plus an audit note per converted symbol.
struct PricedQuotes {
    quotes: Vec<MarketQuote>,
    asset_notes: HashMap<String, String>,
    /// This round's USD/BRL rate, if PTAX answered.
    usd_brl: Option<FxRate>,
}

/// The prices of one refresh round, shared by every portfolio.
struct PriceBook {
    quotes: PricedQuotes,
    /// Every symbol asked for, priced or not: a holding outside this set
    /// (just imported) triggers a refresh.
    asked: BTreeSet<String>,
    priced_at: DateTime<Utc>,
    fetched: Instant,
}

pub struct AllocationsService {
    providers: MarketProviders,
    portfolio: Arc<dyn PortfolioRepo>,
    barca_targets: Arc<dyn BarcaTargetRepo>,
    snapshots: Arc<dyn SnapshotRepo>,
    directory: Arc<dyn PortfolioDirectoryRepo>,
    /// Which BARCA target profile is active ("BullMarket", "BearMarket"...).
    current_market: String,
    refresh_interval: Duration,
    book: RwLock<Option<Arc<PriceBook>>>,
    /// Held while refreshing, so concurrent requests wait for one refresh
    /// instead of each calling the APIs. Holds when and why the last one
    /// failed.
    refresh: Mutex<Option<(Instant, String)>>,
}

impl AllocationsService {
    pub fn new(
        providers: MarketProviders,
        portfolio: Arc<dyn PortfolioRepo>,
        barca_targets: Arc<dyn BarcaTargetRepo>,
        snapshots: Arc<dyn SnapshotRepo>,
        directory: Arc<dyn PortfolioDirectoryRepo>,
        current_market: String,
        refresh_interval: Duration,
    ) -> Self {
        Self {
            providers,
            portfolio,
            barca_targets,
            snapshots,
            directory,
            current_market,
            refresh_interval,
            book: RwLock::new(None),
            refresh: Mutex::new(None),
        }
    }

    /// `portfolio`'s allocations at the current shared prices, refreshing
    /// them first if needed. Target and quantity edits show up at once,
    /// since only the prices are cached.
    ///
    /// When a refresh fails, the last prices are used (their age is in the
    /// result, so the UI can show they are old); only with no prices at
    /// all is it an error.
    pub async fn current_report(
        &self,
        portfolio: PortfolioId,
    ) -> Result<PricedReport, AllocationsError> {
        let positions = self.portfolio.fetch_positions(portfolio).await?;
        let book = self.prices_for(&positions).await?;
        let report = self.compute(portfolio, &positions, &book).await?;
        Ok(PricedReport {
            report,
            usd_brl: book.quotes.usd_brl.clone(),
            prices_as_of: book.priced_at.to_rfc3339(),
            refresh_interval_secs: self.refresh_interval.as_secs(),
        })
    }

    async fn prices_for(
        &self,
        positions: &[WalletPosition],
    ) -> Result<Arc<PriceBook>, AllocationsError> {
        if let Some(book) = self.usable_book(positions).await {
            return Ok(book);
        }
        let mut last_failure = self.refresh.lock().await;
        // Another request may have refreshed while this one waited.
        if let Some(book) = self.usable_book(positions).await {
            return Ok(book);
        }
        let cached = self.book.read().await.clone();
        if let Some((failed_at, reason)) = last_failure.as_ref() {
            if failed_at.elapsed() < RETRY_AFTER_FAILURE {
                // Say why, not just that it failed: this is what the user sees.
                return cached.ok_or_else(|| {
                    AllocationsError::CryptoPrices(
                        format!("{reason} (retrying within a minute)").into(),
                    )
                });
            }
        }
        match self.refresh_all().await {
            Ok(book) => {
                *last_failure = None;
                Ok(book)
            }
            Err(e) => {
                *last_failure = Some((Instant::now(), failure_reason(&e)));
                match cached {
                    Some(old) => {
                        tracing::warn!(
                            error = %e,
                            prices_as_of = %old.priced_at,
                            "Price refresh failed — serving the previous prices"
                        );
                        Ok(old)
                    }
                    None => Err(e),
                }
            }
        }
    }

    /// The cached prices, if fresh enough and covering every holding.
    async fn usable_book(&self, positions: &[WalletPosition]) -> Option<Arc<PriceBook>> {
        let book = self.book.read().await.clone()?;
        let fresh = book.fetched.elapsed() < self.refresh_interval;
        let covered = priced_symbols(positions).is_subset(&book.asked);
        (fresh && covered).then_some(book)
    }

    /// Fetches prices for every portfolio's holdings in one round and
    /// records a snapshot for each portfolio that holds anything.
    async fn refresh_all(&self) -> Result<Arc<PriceBook>, AllocationsError> {
        let started = Instant::now();
        let mut holdings = Vec::new();
        for id in self.directory.list_all_portfolio_ids().await? {
            holdings.push((id, self.portfolio.fetch_positions(id).await?));
        }
        let all: Vec<WalletPosition> = holdings
            .iter()
            .flat_map(|(_, positions)| positions.iter().cloned())
            .collect();

        let book = Arc::new(PriceBook {
            quotes: self.fetch_all_quotes(&all).await?,
            asked: priced_symbols(&all),
            priced_at: Utc::now(),
            fetched: Instant::now(),
        });
        log_unpriced(&holdings, &book);

        let mut recorded = 0;
        for (id, positions) in &holdings {
            if positions.is_empty() {
                continue; // an empty account would only chart a $0 line
            }
            match self.record_snapshot(*id, positions, &book).await {
                Ok(()) => recorded += 1,
                Err(e) => tracing::error!(
                    error = %e,
                    portfolio_id = %id,
                    "Failed to record this portfolio's snapshot"
                ),
            }
        }
        *self.book.write().await = Some(book.clone());
        tracing::info!(
            portfolios = holdings.len(),
            snapshots = recorded,
            symbols = book.asked.len(),
            priced = book.quotes.quotes.len(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            market = %self.current_market,
            "Refreshed shared prices"
        );
        Ok(book)
    }

    async fn compute(
        &self,
        portfolio: PortfolioId,
        positions: &[WalletPosition],
        book: &PriceBook,
    ) -> Result<AllocationReport, AllocationsError> {
        let barca_targets: HashMap<String, f64> = self
            .barca_targets
            .fetch_barca_targets(portfolio, &self.current_market)
            .await?
            .into_iter()
            .map(|t| (t.barca, t.target_percent))
            .collect();
        Ok(compute_allocations(
            positions,
            &book.quotes.quotes,
            &barca_targets,
        ))
    }

    async fn record_snapshot(
        &self,
        portfolio: PortfolioId,
        positions: &[WalletPosition],
        book: &PriceBook,
    ) -> Result<(), AllocationsError> {
        let report = self.compute(portfolio, positions, book).await?;
        self.snapshots
            .record_snapshot(
                portfolio,
                &AllocationSnapshot {
                    timestamp: book.priced_at.to_rfc3339(),
                    report,
                    asset_notes: book.quotes.asset_notes.clone(),
                },
            )
            .await?;
        Ok(())
    }

    /// Crypto prices are required. A Brazilian/US equity provider failing
    /// only leaves those assets out of this round (logged), since they are
    /// optional extras the user may not have API keys for. The same goes
    /// for the PTAX rate: without it, Brazilian assets are left out rather
    /// than valued in reais as if they were dollars.
    async fn fetch_all_quotes(
        &self,
        positions: &[WalletPosition],
    ) -> Result<PricedQuotes, AllocationsError> {
        let crypto_symbols = symbols_of_class(positions, AssetClass::Crypto);
        let mut quotes = if crypto_symbols.is_empty() {
            Vec::new()
        } else {
            self.providers
                .crypto
                .fetch_quotes(&crypto_symbols)
                .await
                .map_err(AllocationsError::CryptoPrices)?
        };
        let mut asset_notes = HashMap::new();

        // Fetched every round, not only when there are Brazilian assets:
        // the UI also uses it to show the total in reais.
        let usd_brl = match self.providers.usd_brl.fetch_usd_brl().await {
            Ok(fx) => Some(fx),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch the PTAX rate");
                None
            }
        };

        let mut br_quotes = self
            .fetch_equity_quotes(
                positions,
                AssetClass::BrEquities,
                &self.providers.br_equities,
            )
            .await;
        br_quotes.extend(manual_brl_quotes(positions));
        if !br_quotes.is_empty() {
            match &usd_brl {
                Some(fx) => {
                    tracing::info!(
                        brl_per_usd = fx.brl_per_usd,
                        ptax_quoted_at = %fx.quoted_at,
                        symbols = br_quotes.len(),
                        "Converting Brazilian quotes from BRL to USD (PTAX venda)"
                    );
                    let converted = convert_brl_to_usd(br_quotes, fx);
                    quotes.extend(converted.quotes);
                    asset_notes.extend(converted.asset_notes);
                }
                None => tracing::error!(
                    symbols = ?br_quotes.iter().map(|q| &q.symbol).collect::<Vec<_>>(),
                    "No PTAX rate — Brazilian assets are left out of this refresh"
                ),
            }
        }

        quotes.extend(
            self.fetch_equity_quotes(positions, AssetClass::UsIndices, &self.providers.us_indices)
                .await,
        );
        Ok(PricedQuotes {
            quotes,
            asset_notes,
            usd_brl,
        })
    }

    /// Quotes for one optional asset class. A provider failure is logged
    /// and yields no quotes, so it never aborts the whole refresh.
    async fn fetch_equity_quotes(
        &self,
        positions: &[WalletPosition],
        class: AssetClass,
        provider: &Arc<dyn EquityProvider>,
    ) -> Vec<MarketQuote> {
        let symbols = symbols_of_class(positions, class);
        if symbols.is_empty() {
            return Vec::new();
        }
        match provider.fetch_quotes(&symbols).await {
            Ok(quotes) => quotes,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    asset_class = class.as_str(),
                    ?symbols,
                    "Failed to fetch quotes — these assets are left out of this refresh"
                );
                Vec::new()
            }
        }
    }
}

/// The provider's own message, without our "failed to fetch…" prefix,
/// which the error adds again when it is shown.
fn failure_reason(e: &AllocationsError) -> String {
    match e {
        AllocationsError::CryptoPrices(inner) | AllocationsError::Repo(inner) => inner.to_string(),
    }
}

/// Every symbol a provider prices (crypto, Brazilian equities, US indices).
fn priced_symbols(positions: &[WalletPosition]) -> BTreeSet<String> {
    positions
        .iter()
        .filter(|p| AssetClass::parse(&p.asset_class).is_some())
        .map(|p| p.symbol.clone())
        .collect()
}

/// Divides each BRL price by the PTAX rate, and keeps the original price and
/// the rate used as a JSON note, so any recorded USD price can be traced
/// back to its BRL quote.
fn convert_brl_to_usd(brl_quotes: Vec<MarketQuote>, fx: &FxRate) -> PricedQuotes {
    let mut asset_notes = HashMap::new();
    let quotes = brl_quotes
        .into_iter()
        .map(|quote| {
            let note = serde_json::json!({
                "price_brl": quote.price,
                "ptax_venda": fx.brl_per_usd,
                "ptax_quoted_at": fx.quoted_at,
            });
            asset_notes.insert(quote.symbol.clone(), note.to_string());
            MarketQuote {
                price: quote.price / fx.brl_per_usd,
                symbol: quote.symbol,
            }
        })
        .collect();
    PricedQuotes {
        quotes,
        asset_notes,
        usd_brl: Some(fx.clone()),
    }
}

/// A `brl-manual` holding's quantity is its value in reais, so it is quoted
/// at R$1 — then converted with PTAX like any other Brazilian quote.
fn manual_brl_quotes(positions: &[WalletPosition]) -> Vec<MarketQuote> {
    symbols_of_class(positions, AssetClass::BrlManual)
        .into_iter()
        .map(|symbol| MarketQuote { symbol, price: 1.0 })
        .collect()
}

/// One warning per portfolio holding something this round couldn't price
/// (it is left out of that portfolio's totals until it gets a price).
fn log_unpriced(holdings: &[(PortfolioId, Vec<WalletPosition>)], book: &PriceBook) {
    let priced: BTreeSet<&str> = book
        .quotes
        .quotes
        .iter()
        .map(|q| q.symbol.as_str())
        .collect();
    for (id, positions) in holdings {
        let missing: Vec<String> = positions
            .iter()
            .filter(|p| p.current_quantity > 0.0 && !priced.contains(p.symbol.as_str()))
            .map(|p| format!("{} ({})", p.symbol, p.asset_class))
            .collect();
        if !missing.is_empty() {
            tracing::warn!(
                portfolio_id = %id,
                ?missing,
                "No price for these holdings — left out of the totals"
            );
        }
    }
}

/// Distinct symbols of one asset class, sorted.
fn symbols_of_class(positions: &[WalletPosition], class: AssetClass) -> Vec<String> {
    positions
        .iter()
        .filter(|p| p.asset_class == class.as_str())
        .map(|p| p.symbol.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::market_data::{FakeFx, FakeProvider};
    use crate::domain::models::{LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PositionKey};
    use crate::domain::repository::{NewUser, UserRepo};
    use crate::infra::sqlite::{SqliteRepo, test_repo};

    /// These tests are about one wallet; isolation between portfolios is
    /// tested in `infra::sqlite::repo` and `tests::integration`.
    const PORTFOLIO: PortfolioId = PortfolioId::FIRST;

    async fn seed(repo: &SqliteRepo, symbol: &str, asset_class: &str, qty: f64, target: f64) {
        seed_in(repo, PORTFOLIO, symbol, asset_class, qty, target).await;
    }

    async fn seed_in(
        repo: &SqliteRepo,
        portfolio: PortfolioId,
        symbol: &str,
        asset_class: &str,
        qty: f64,
        target: f64,
    ) {
        let entry = LedgerEntry {
            symbol: symbol.to_string(),
            group_name: Some("Core".to_string()),
            barca: Some("Base".to_string()),
            asset_class: asset_class.to_string(),
            current_quantity: qty,
            last_price: None,
            notes: None,
        };
        let target = NewPortfolioTarget {
            key: PositionKey {
                symbol: symbol.to_string(),
                group_name: "Core".to_string(),
                barca: "Base".to_string(),
                asset_class: asset_class.to_string(),
            },
            target_percent: target,
        };
        repo.import_positions(portfolio, &[target], &[entry])
            .await
            .unwrap();
    }

    async fn set_barca_target(repo: &SqliteRepo, market: &str, target_percent: f64) {
        repo.replace_barca_targets(
            PORTFOLIO,
            market,
            &[NewBarcaTarget {
                barca: "Base".to_string(),
                target_percent,
            }],
            None,
        )
        .await
        .unwrap();
    }

    /// Refreshes on every request, like the old "Update Prices".
    fn service(repo: Arc<SqliteRepo>, providers: MarketProviders) -> AllocationsService {
        service_refreshing_every(repo, providers, Duration::ZERO)
    }

    fn service_refreshing_every(
        repo: Arc<SqliteRepo>,
        providers: MarketProviders,
        interval: Duration,
    ) -> AllocationsService {
        AllocationsService::new(
            providers,
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo,
            "BullMarket".to_string(),
            interval,
        )
    }

    fn providers(crypto: FakeProvider, br: FakeProvider) -> MarketProviders {
        providers_with_fx(crypto, br, FakeFx(Some(5.0)))
    }

    fn providers_with_fx(crypto: FakeProvider, br: FakeProvider, fx: FakeFx) -> MarketProviders {
        MarketProviders {
            crypto: Arc::new(crypto),
            br_equities: Arc::new(br),
            us_indices: Arc::new(FakeProvider::empty()),
            usd_brl: Arc::new(fx),
        }
    }

    #[tokio::test]
    async fn merges_crypto_and_equity_prices_and_records_a_snapshot() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        seed(&repo, "HGRU11", "br-equities", 10.0, 50.0).await;
        set_barca_target(&repo, "BullMarket", 100.0).await;

        let report = service(
            repo.clone(),
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::with_prices(&[("HGRU11", 100.0)]),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        // HGRU11: 10 × (R$100 / 5.0) = $200, plus BTC $10.
        assert_eq!(report.total_value, 210.0);
        assert_eq!(report.per_asset.len(), 2);
        let totals = repo.fetch_total_history(PORTFOLIO).await.unwrap();
        assert_eq!(totals.len(), 1, "one snapshot recorded");
        assert_eq!(totals[0].total_value, Some(210.0));
    }

    #[tokio::test]
    async fn brazilian_prices_are_converted_to_usd_but_crypto_is_not() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        seed(&repo, "HGRU11", "br-equities", 1.0, 50.0).await;

        let report = service(
            repo,
            providers_with_fx(
                FakeProvider::with_prices(&[("BTC", 100.0)]),
                FakeProvider::with_prices(&[("HGRU11", 104.0)]),
                FakeFx(Some(5.2)),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        let price = |symbol: &str| {
            report
                .per_asset
                .iter()
                .find(|a| a.symbol == symbol)
                .unwrap()
                .price
        };
        assert_eq!(price("BTC"), 100.0);
        assert!((price("HGRU11") - 20.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn without_ptax_brazilian_assets_are_left_out_not_valued_in_brl() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        seed(&repo, "HGRU11", "br-equities", 10.0, 50.0).await;

        let report = service(
            repo.clone(),
            providers_with_fx(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::with_prices(&[("HGRU11", 100.0)]),
                FakeFx(None),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        let symbols: Vec<_> = report.per_asset.iter().map(|a| a.symbol.as_str()).collect();
        assert_eq!(symbols, ["BTC"]);
        assert_eq!(report.total_value, 10.0);
        assert_eq!(repo.fetch_total_history(PORTFOLIO).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_brl_manual_holding_is_its_quantity_in_reais_without_any_api() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        seed(&repo, "FUND A", "brl-manual", 5_500.0, 50.0).await;

        // brapi failing proves the manual value doesn't depend on it.
        let report = service(
            repo,
            providers_with_fx(
                FakeProvider::with_prices(&[("BTC", 1_000.0)]),
                FakeProvider::failing(),
                FakeFx(Some(5.5)),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        // R$5,500 / 5.5 = $1,000.
        let fund = report
            .per_asset
            .iter()
            .find(|a| a.symbol == "FUND A")
            .expect("valued, not dropped");
        assert!((fund.value - 1_000.0).abs() < 1e-9);
        assert!((report.total_value - 2_000.0).abs() < 1e-9);
        assert!(report.unpriced.is_empty());
    }

    #[tokio::test]
    async fn without_ptax_a_brl_manual_holding_is_listed_as_unpriced() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        seed(&repo, "CASH BRL", "brl-manual", 500.0, 50.0).await;

        let report = service(
            repo,
            providers_with_fx(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::empty(),
                FakeFx(None),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        assert_eq!(
            report.total_value, 10.0,
            "never valued as if reais were dollars"
        );
        let unpriced: Vec<_> = report.unpriced.iter().map(|u| u.symbol.as_str()).collect();
        assert_eq!(unpriced, ["CASH BRL"]);
    }

    #[tokio::test]
    async fn a_name_brapi_does_not_know_is_reported_as_unpriced() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        seed(&repo, "BOND B", "br-equities", 40.0, 50.0).await;

        let report = service(
            repo,
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::empty(),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        assert_eq!(report.unpriced.len(), 1);
        assert_eq!(report.unpriced[0].symbol, "BOND B");
        assert_eq!(report.unpriced[0].asset_class, "br-equities");
    }

    #[tokio::test]
    async fn returns_the_usd_brl_rate_even_without_brazilian_assets() {
        // The UI shows the total in reais with it (it used a fixed 5.6).
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;

        let priced = service(
            repo,
            providers_with_fx(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::empty(),
                FakeFx(Some(5.2)),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap();

        let fx = priced.usd_brl.clone().expect("rate included");
        assert_eq!(fx.brl_per_usd, 5.2);
        assert_eq!(fx.quoted_at, "2026-10-01 13:10:35");
        let json = serde_json::to_value(&priced).unwrap();
        assert_eq!(
            json["total_value"], 10.0,
            "report fields stay at the top level"
        );
        assert_eq!(json["usd_brl"]["brl_per_usd"], 5.2);
    }

    #[tokio::test]
    async fn without_ptax_the_rate_is_none_and_the_refresh_still_succeeds() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;

        let priced = service(
            repo,
            providers_with_fx(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::empty(),
                FakeFx(None),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap();

        assert_eq!(priced.usd_brl, None);
        assert_eq!(priced.report.total_value, 10.0);
        assert!(serde_json::to_value(&priced).unwrap()["usd_brl"].is_null());
    }

    #[test]
    fn convert_brl_to_usd_divides_by_ptax_and_records_an_audit_note() {
        let fx = FxRate {
            brl_per_usd: 5.0,
            quoted_at: "2026-10-01 13:10:35".into(),
        };
        let converted = convert_brl_to_usd(
            vec![MarketQuote {
                symbol: "XPML11".into(),
                price: 101.35,
            }],
            &fx,
        );

        assert!((converted.quotes[0].price - 20.27).abs() < 1e-9);
        let note: serde_json::Value =
            serde_json::from_str(&converted.asset_notes["XPML11"]).unwrap();
        assert_eq!(note["price_brl"], 101.35);
        assert_eq!(note["ptax_venda"], 5.0);
        assert_eq!(note["ptax_quoted_at"], "2026-10-01 13:10:35");
    }

    #[test]
    fn convert_brl_to_usd_with_no_quotes_is_empty() {
        let fx = FxRate {
            brl_per_usd: 5.0,
            quoted_at: String::new(),
        };
        let converted = convert_brl_to_usd(vec![], &fx);
        assert!(converted.quotes.is_empty());
        assert!(converted.asset_notes.is_empty());
    }

    #[tokio::test]
    async fn an_equity_provider_failure_does_not_abort_the_refresh() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        seed(&repo, "HGRU11", "br-equities", 10.0, 0.0).await;

        let report = service(
            repo,
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::failing(),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        let symbols: Vec<_> = report.per_asset.iter().map(|a| a.symbol.as_str()).collect();
        assert_eq!(symbols, ["BTC"]);
    }

    #[tokio::test]
    async fn a_crypto_provider_failure_is_a_typed_error_and_records_nothing() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;

        let err = service(
            repo.clone(),
            providers(FakeProvider::failing(), FakeProvider::empty()),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap_err();

        assert!(matches!(err, AllocationsError::CryptoPrices(_)));
        assert!(
            repo.fetch_total_history(PORTFOLIO)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn uses_only_the_current_markets_barca_targets() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        set_barca_target(&repo, "BullMarket", 100.0).await;
        set_barca_target(&repo, "BearMarket", 40.0).await;

        let report = service(
            repo,
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0)]),
                FakeProvider::empty(),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;

        assert_eq!(report.per_barca[0].target_percent, 100.0);
    }

    #[test]
    fn symbols_of_class_filters_dedupes_and_sorts() {
        let position = |symbol: &str, class: &str| WalletPosition {
            symbol: symbol.to_string(),
            group_name: None,
            barca: None,
            asset_class: class.to_string(),
            target_percent: 0.0,
            current_quantity: 1.0,
            last_price: None,
            notes: None,
            source_count: 1,
            removed: false,
        };
        let positions = vec![
            position("XPML11", "br-equities"),
            position("HGRU11", "br-equities"),
            position("HGRU11", "br-equities"),
            position("BTC", "crypto"),
        ];
        assert_eq!(
            symbols_of_class(&positions, AssetClass::BrEquities),
            ["HGRU11", "XPML11"]
        );
    }

    // --- shared, cached prices -------------------------------------------

    /// A crypto provider whose failure can be switched on, counting calls.
    struct SwitchableCrypto {
        inner: FakeProvider,
        down: std::sync::atomic::AtomicBool,
    }

    #[async_trait::async_trait]
    impl CryptoProvider for SwitchableCrypto {
        async fn fetch_quotes(
            &self,
            symbols: &[String],
        ) -> crate::domain::market_data::MarketDataResult<Vec<MarketQuote>> {
            let quotes = CryptoProvider::fetch_quotes(&self.inner, symbols).await?;
            if self.down.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("CoinMarketCap is down".into());
            }
            Ok(quotes)
        }
    }

    fn switchable(prices: &[(&str, f64)]) -> Arc<SwitchableCrypto> {
        Arc::new(SwitchableCrypto {
            inner: FakeProvider::with_prices(prices),
            down: Default::default(),
        })
    }

    fn providers_with_crypto(crypto: Arc<dyn CryptoProvider>) -> MarketProviders {
        MarketProviders {
            crypto,
            br_equities: Arc::new(FakeProvider::empty()),
            us_indices: Arc::new(FakeProvider::empty()),
            usd_brl: Arc::new(FakeFx(Some(5.0))),
        }
    }

    const HOUR: Duration = Duration::from_secs(3600);

    #[tokio::test]
    async fn polling_within_the_interval_reuses_the_prices_and_records_once() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        let crypto = switchable(&[("BTC", 10.0)]);
        let service =
            service_refreshing_every(repo.clone(), providers_with_crypto(crypto.clone()), HOUR);

        let first = service.current_report(PORTFOLIO).await.unwrap();
        let second = service.current_report(PORTFOLIO).await.unwrap();

        assert_eq!(crypto.inner.calls(), 1, "one API call for two polls");
        assert_eq!(first.prices_as_of, second.prices_as_of);
        assert_eq!(second.refresh_interval_secs, 3600);
        assert_eq!(repo.fetch_total_history(PORTFOLIO).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn only_held_crypto_symbols_are_asked_for() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        let report = service(
            repo,
            providers(
                FakeProvider::with_prices(&[("BTC", 10.0), ("DOGE", 1.0)]),
                FakeProvider::empty(),
            ),
        )
        .current_report(PORTFOLIO)
        .await
        .unwrap()
        .report;
        let symbols: Vec<_> = report.per_asset.iter().map(|a| a.symbol.as_str()).collect();
        assert_eq!(symbols, ["BTC"]);
    }

    #[tokio::test]
    async fn edits_show_up_at_once_without_new_prices() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        let crypto = switchable(&[("BTC", 10.0)]);
        let service =
            service_refreshing_every(repo.clone(), providers_with_crypto(crypto.clone()), HOUR);
        service.current_report(PORTFOLIO).await.unwrap();

        // A second BTC source: the quantity changes, the prices don't.
        repo.import_positions(
            PORTFOLIO,
            &[],
            &[LedgerEntry {
                symbol: "BTC".to_string(),
                group_name: Some("Core".to_string()),
                barca: Some("Base".to_string()),
                asset_class: "crypto".to_string(),
                current_quantity: 2.0,
                last_price: None,
                notes: Some("exchange".to_string()),
            }],
        )
        .await
        .unwrap();
        let after = service.current_report(PORTFOLIO).await.unwrap().report;

        assert_eq!(after.total_value, 30.0, "1 + 2 BTC at $10");
        assert_eq!(crypto.inner.calls(), 1);
    }

    #[tokio::test]
    async fn a_newly_held_symbol_triggers_a_refresh_even_when_prices_are_fresh() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 50.0).await;
        let crypto = switchable(&[("BTC", 10.0), ("ETH", 5.0)]);
        let service =
            service_refreshing_every(repo.clone(), providers_with_crypto(crypto.clone()), HOUR);
        service.current_report(PORTFOLIO).await.unwrap();

        seed(&repo, "ETH", "crypto", 2.0, 50.0).await; // e.g. a CSV import
        let report = service.current_report(PORTFOLIO).await.unwrap().report;

        assert_eq!(crypto.inner.calls(), 2);
        assert_eq!(report.total_value, 20.0, "ETH is priced right away");
    }

    #[tokio::test]
    async fn one_refresh_prices_and_records_every_portfolio_that_holds_something() {
        let repo = test_repo().await;
        let owner = repo
            .create_user(NewUser {
                username: "owner",
                password_hash: "hash",
                role: "manager",
                email: "owner@example.com",
                phone: None,
            })
            .await
            .unwrap();
        let spouse = repo
            .create_portfolio("Spouse", owner.id.unwrap(), None)
            .await
            .unwrap();
        let empty = repo
            .create_portfolio("Empty", owner.id.unwrap(), None)
            .await
            .unwrap();
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        seed_in(&repo, spouse, "ETH", "crypto", 3.0, 100.0).await;
        let crypto = switchable(&[("BTC", 10.0), ("ETH", 5.0)]);
        let service =
            service_refreshing_every(repo.clone(), providers_with_crypto(crypto.clone()), HOUR);

        service.current_report(PORTFOLIO).await.unwrap();
        let spouse_report = service.current_report(spouse).await.unwrap().report;

        assert_eq!(crypto.inner.calls(), 1, "both portfolios share one call");
        assert_eq!(spouse_report.total_value, 15.0);
        assert_eq!(
            spouse_report.per_asset.len(),
            1,
            "no BTC from the other wallet"
        );
        assert_eq!(repo.fetch_total_history(spouse).await.unwrap().len(), 1);
        assert!(
            repo.fetch_total_history(empty).await.unwrap().is_empty(),
            "an empty account gets no $0 snapshot"
        );
    }

    #[tokio::test]
    async fn a_failed_refresh_serves_the_previous_prices() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        let crypto = switchable(&[("BTC", 10.0)]);
        let service = service(repo.clone(), providers_with_crypto(crypto.clone()));
        let first = service.current_report(PORTFOLIO).await.unwrap();

        crypto.down.store(true, std::sync::atomic::Ordering::SeqCst);
        let during_outage = service.current_report(PORTFOLIO).await.unwrap();

        assert_eq!(during_outage.report.total_value, 10.0);
        assert_eq!(
            during_outage.prices_as_of, first.prices_as_of,
            "the age shows the prices are old"
        );
        assert_eq!(repo.fetch_total_history(PORTFOLIO).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn after_a_failure_the_apis_are_not_called_again_right_away() {
        let repo = test_repo().await;
        seed(&repo, "BTC", "crypto", 1.0, 100.0).await;
        let crypto = switchable(&[("BTC", 10.0)]);
        crypto.down.store(true, std::sync::atomic::Ordering::SeqCst);
        let service = service(repo, providers_with_crypto(crypto.clone()));

        for _ in 0..3 {
            let err = service.current_report(PORTFOLIO).await.unwrap_err();
            assert!(matches!(err, AllocationsError::CryptoPrices(_)), "{err}");
            // Regression: it said only "the last attempt failed".
            assert!(err.to_string().contains("CoinMarketCap is down"), "{err}");
        }
        assert_eq!(crypto.inner.calls(), 1, "polls during the retry delay wait");
    }
}
