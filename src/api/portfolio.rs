//! Portfolio endpoints: live allocations, history, targets and CSV upload.

use crate::api::AppState;
use crate::api::auth::CurrentUser;
use crate::api::error::{ApiError, ApiResult};
use crate::domain::models::{AllocationReport, BarcaTarget, NewBarcaTarget, Role, WalletPosition};
use crate::usecases::history_service::HistoryLevel;
use crate::usecases::targets_service::{QuantityCorrection, TargetRow};
use axum::Json;
use axum::extract::{Multipart, Query, State};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

/// "Update Prices": fetch live prices, recompute and record a snapshot.
#[tracing::instrument(skip_all, fields(user_id = current.user_id))]
pub async fn allocations(
    current: CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<AllocationReport>> {
    Ok(Json(state.allocations.refresh_prices().await?))
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    level: Option<String>,
}

pub async fn history(
    _current: CurrentUser,
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
        .fetch_history(level)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(history))
}

pub async fn get_portfolio_targets(
    _current: CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<WalletPosition>>> {
    Ok(Json(state.targets.list_positions().await?))
}

#[derive(Deserialize)]
pub struct SavePortfolioTargetsPayload {
    rows: Vec<TargetRow>,
}

#[tracing::instrument(skip_all, fields(user_id = current.user_id, rows = payload.rows.len()))]
pub async fn save_portfolio_targets(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<SavePortfolioTargetsPayload>,
) -> ApiResult<Json<Value>> {
    current.require(Role::Manager)?;
    let row_count = payload.rows.len();
    state
        .targets
        .save_portfolio_targets(payload.rows, Some(current.user_id))
        .await?;
    info!("Saved portfolio targets");
    Ok(Json(json!({ "ok": true, "rows": row_count })))
}

#[tracing::instrument(skip_all, fields(user_id = current.user_id, symbol = %payload.symbol))]
pub async fn correct_quantity(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<QuantityCorrection>,
) -> ApiResult<Json<Value>> {
    current.require(Role::Manager)?;
    let new_quantity = payload.current_quantity;
    state.targets.correct_quantity(payload).await?;
    info!(new_quantity, "Corrected position quantity");
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct BarcaTargetsQuery {
    market: String,
}

pub async fn get_barca_targets(
    _current: CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<BarcaTargetsQuery>,
) -> ApiResult<Json<Vec<BarcaTarget>>> {
    Ok(Json(state.targets.list_barca_targets(&query.market).await?))
}

#[derive(Deserialize)]
pub struct SaveBarcaTargetsPayload {
    market: String,
    targets: Vec<NewBarcaTarget>,
}

#[tracing::instrument(skip_all, fields(user_id = current.user_id, market = %payload.market))]
pub async fn save_barca_targets(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<SaveBarcaTargetsPayload>,
) -> ApiResult<Json<Value>> {
    current.require(Role::Manager)?;
    state
        .targets
        .save_barca_targets(&payload.market, &payload.targets, Some(current.user_id))
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

#[tracing::instrument(skip_all, fields(user_id = current.user_id))]
pub async fn upload_wallet_csv(
    current: CurrentUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<Json<Value>> {
    current.require(Role::Manager)?;
    let csv = read_csv_upload(&mut multipart).await?;
    let imported = state.wallet_import.import_csv(csv.as_slice()).await?;
    info!(imported, bytes = csv.len(), "Imported wallet CSV upload");
    Ok(Json(json!({ "imported": imported })))
}
