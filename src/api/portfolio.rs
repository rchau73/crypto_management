//! Endpoints inside one portfolio (`/api/portfolios/{id}/...`): live
//! allocations, history, targets and CSV upload/download. Each handler takes
//! a `PortfolioScope`, so it only runs for a member of that portfolio and
//! passes `scope.portfolio` to the service.

use crate::api::AppState;
use crate::api::error::{ApiError, ApiResult};
use crate::api::portfolios::PortfolioScope;
use crate::domain::models::{
    BarcaTarget, NewBarcaTarget, PositionSource, PricedReport, WalletPosition,
};
use crate::usecases::history_service::HistoryLevel;
use crate::usecases::targets_service::{QuantityCorrection, TargetRow};
use crate::usecases::trade_service::{
    TradeOutcome, TradeRequest, TransferOutcome, TransferRequest,
};
use crate::usecases::wallet_import::ImportSummary;
use axum::Json;
use axum::extract::{Multipart, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

/// The live allocations, at the shared prices (refreshed when they are
/// older than `PRICE_REFRESH_MINUTES`). The app polls this.
#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio))]
pub async fn allocations(
    scope: PortfolioScope,
    State(state): State<AppState>,
) -> ApiResult<Json<PricedReport>> {
    Ok(Json(
        state.allocations.current_report(scope.portfolio).await?,
    ))
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    level: Option<String>,
}

pub async fn history(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> ApiResult<Json<Value>> {
    let level_name = query.level.as_deref().unwrap_or("totals");
    let level = HistoryLevel::parse(level_name).ok_or_else(|| {
        ApiError::bad_request(format!(
            "unknown history level \"{level_name}\" (expected totals, assets, groups or barca)"
        ))
    })?;
    let history = state
        .history
        .fetch_history(scope.portfolio, level)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(history))
}

pub async fn get_portfolio_targets(
    scope: PortfolioScope,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<WalletPosition>>> {
    Ok(Json(state.targets.list_positions(scope.portfolio).await?))
}

#[derive(Deserialize)]
pub struct SavePortfolioTargetsPayload {
    rows: Vec<TargetRow>,
}

#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio, rows = payload.rows.len()))]
pub async fn save_portfolio_targets(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Json(payload): Json<SavePortfolioTargetsPayload>,
) -> ApiResult<Json<Value>> {
    scope.require_manage()?;
    let row_count = payload.rows.len();
    state
        .targets
        .save_portfolio_targets(scope.portfolio, payload.rows, Some(scope.user.user_id))
        .await?;
    info!("Saved portfolio targets");
    Ok(Json(json!({ "ok": true, "rows": row_count })))
}

#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio, symbol = %payload.symbol))]
pub async fn correct_quantity(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Json(payload): Json<QuantityCorrection>,
) -> ApiResult<Json<Value>> {
    scope.require_manage()?;
    let new_quantity = payload.current_quantity;
    state
        .targets
        .correct_quantity(scope.portfolio, payload)
        .await?;
    info!(new_quantity, "Corrected position quantity");
    Ok(Json(json!({ "ok": true })))
}

/// Every source (exchange/wallet) of every position, with its quantity.
pub async fn list_sources(
    scope: PortfolioScope,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<PositionSource>>> {
    Ok(Json(state.trades.list_sources(scope.portfolio).await?))
}

/// "Record trade": a buy or sell made on an exchange, and the cash it
/// moved, written straight into the wallet (no CSV round trip).
#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio, symbol = %payload.asset.symbol))]
pub async fn record_trade(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Json(payload): Json<TradeRequest>,
) -> ApiResult<Json<TradeOutcome>> {
    scope.require_manage()?;
    Ok(Json(
        state
            .trades
            .record(scope.portfolio, payload, Some(scope.user.user_id))
            .await?,
    ))
}

/// "Move balance": a quantity moved to another BARCA/group and/or exchange
/// (e.g. 0.5 BTC from Binance to Deribit), with no trade.
#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio, symbol = %payload.from.symbol))]
pub async fn record_transfer(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Json(payload): Json<TransferRequest>,
) -> ApiResult<Json<TransferOutcome>> {
    scope.require_manage()?;
    Ok(Json(
        state
            .trades
            .transfer(scope.portfolio, payload, Some(scope.user.user_id))
            .await?,
    ))
}

#[derive(Deserialize)]
pub struct BarcaTargetsQuery {
    market: String,
}

pub async fn get_barca_targets(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Query(query): Query<BarcaTargetsQuery>,
) -> ApiResult<Json<Vec<BarcaTarget>>> {
    Ok(Json(
        state
            .targets
            .list_barca_targets(scope.portfolio, &query.market)
            .await?,
    ))
}

#[derive(Deserialize)]
pub struct SaveBarcaTargetsPayload {
    market: String,
    targets: Vec<NewBarcaTarget>,
}

#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio, market = %payload.market))]
pub async fn save_barca_targets(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Json(payload): Json<SaveBarcaTargetsPayload>,
) -> ApiResult<Json<Value>> {
    scope.require_manage()?;
    state
        .targets
        .save_barca_targets(
            scope.portfolio,
            &payload.market,
            &payload.targets,
            Some(scope.user.user_id),
        )
        .await?;
    info!(targets = payload.targets.len(), "Saved BARCA targets");
    Ok(Json(
        json!({ "ok": true, "targets": payload.targets.len() }),
    ))
}

/// Reads the multipart "file" field, rejecting anything that obviously
/// isn't a text CSV (by file name first, then by content).
async fn read_csv_upload(multipart: &mut Multipart) -> ApiResult<Vec<u8>> {
    let bad_upload = |e: axum::extract::multipart::MultipartError| {
        ApiError::bad_request(format!("Bad upload: {e}"))
    };
    while let Some(field) = multipart.next_field().await.map_err(bad_upload)? {
        if field.name() != Some("file") {
            continue;
        }
        if let Some(name) = field.file_name().map(str::to_lowercase) {
            if !(name.ends_with(".csv") || name.ends_with(".txt")) {
                return Err(ApiError::bad_request(format!(
                    "Expected a .csv file, got \"{name}\""
                )));
            }
        }
        let bytes = field.bytes().await.map_err(bad_upload)?;
        // Word, PDF, images... all contain NUL bytes or invalid UTF-8
        // almost immediately; a real CSV never does.
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            return Err(ApiError::bad_request(
                "File does not look like a text/CSV file (binary content detected)",
            ));
        }
        return Ok(bytes.to_vec());
    }
    Err(ApiError::bad_request("Missing \"file\" field in upload"))
}

#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio))]
pub async fn upload_wallet_csv(
    scope: PortfolioScope,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<Json<ImportSummary>> {
    scope.require_manage()?;
    let csv = read_csv_upload(&mut multipart).await?;
    let summary = state
        .wallet_import
        .import_csv(scope.portfolio, csv.as_slice())
        .await?;
    info!(bytes = csv.len(), "Imported wallet CSV upload");
    Ok(Json(summary))
}

/// "Export Wallet CSV": the wallet in the import format, as a download.
#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio))]
pub async fn export_wallet_csv(
    scope: PortfolioScope,
    State(state): State<AppState>,
) -> ApiResult<([(axum::http::HeaderName, String); 2], String)> {
    scope.require_manage()?;
    let csv = state
        .wallet_export
        .export_csv(scope.portfolio)
        .await
        .map_err(ApiError::internal)?;
    let filename = format!(
        "wallet_allocations-{}.csv",
        chrono::Utc::now().format("%Y-%m-%d")
    );
    Ok((
        [
            (CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        csv,
    ))
}
