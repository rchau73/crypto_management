//! Axum glue for the editable-table + Save-all admin UI over portfolio
//! (wallet_allocations) and BARCA targets. Kept out of `main.rs` for the
//! same reason as `auth_handlers.rs` — that file stays composition-root-sized.

use crate::AppState;
use crate::domain::models::{Role, WalletAllocation};
use crate::domain::repository::{BarcaTargetInput, BarcaTargetRepo, HistoryRepo};
use crate::usecases::targets_service::{TargetsService, TargetsServiceError};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{error, info, warn};

use crate::auth_handlers::{CurrentUser, require_role};

type ApiError = (StatusCode, Json<Value>);

fn error_response(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": message.into() })))
}

fn targets_service(state: &AppState) -> TargetsService {
    TargetsService::new(
        state.history_repo.clone(),
        state.history_repo.clone(),
        state.history_repo.clone(),
    )
}

fn targets_service_error_response(e: TargetsServiceError) -> ApiError {
    match e {
        TargetsServiceError::PercentSumMismatch { .. }
        | TargetsServiceError::InvalidAssetClass(_) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        TargetsServiceError::Repo(_) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

pub async fn get_portfolio_targets_handler(
    _current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = state
        .history_repo
        .fetch_current_wallet_allocations()
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!(rows)))
}

#[derive(Deserialize)]
pub struct SavePortfolioTargetsPayload {
    rows: Vec<WalletAllocation>,
}

#[tracing::instrument(skip(state, payload))]
pub async fn save_portfolio_targets_handler(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<SavePortfolioTargetsPayload>,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Manager)?;
    let row_count = payload.rows.len();
    match targets_service(&state)
        .save_portfolio_targets(payload.rows, Some(current.user_id))
        .await
    {
        Ok(()) => {
            info!(
                rows = row_count,
                by_user_id = current.user_id,
                "Saved portfolio targets"
            );
            Ok(Json(json!({ "ok": true, "rows": row_count })))
        }
        Err(e) => {
            warn!(error = %e, by_user_id = current.user_id, "Rejected portfolio targets save");
            Err(targets_service_error_response(e))
        }
    }
}

#[derive(Deserialize)]
pub struct BarcaTargetsQuery {
    market: String,
}

pub async fn get_barca_targets_handler(
    _current: CurrentUser,
    State(state): State<AppState>,
    Query(q): Query<BarcaTargetsQuery>,
) -> Result<Json<Value>, ApiError> {
    let rows = state
        .history_repo
        .fetch_barca_targets(&q.market)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!(rows)))
}

#[derive(Deserialize)]
pub struct BarcaTargetRow {
    barca: String,
    target_percent: f64,
}

#[derive(Deserialize)]
pub struct SaveBarcaTargetsPayload {
    market: String,
    targets: Vec<BarcaTargetRow>,
}

#[tracing::instrument(skip(state, payload))]
pub async fn save_barca_targets_handler(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<SaveBarcaTargetsPayload>,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Manager)?;
    let inputs: Vec<BarcaTargetInput> = payload
        .targets
        .iter()
        .map(|t| BarcaTargetInput {
            barca: &t.barca,
            target_percent: t.target_percent,
        })
        .collect();
    let market = payload.market.clone();
    let target_count = inputs.len();
    match targets_service(&state)
        .save_barca_targets(&payload.market, &inputs, Some(current.user_id))
        .await
    {
        Ok(()) => {
            info!(
                market = %market,
                targets = target_count,
                by_user_id = current.user_id,
                "Saved BARCA targets"
            );
            Ok(Json(json!({ "ok": true, "targets": target_count })))
        }
        Err(e) => {
            error!(error = %e, market = %market, by_user_id = current.user_id, "Rejected BARCA targets save");
            Err(targets_service_error_response(e))
        }
    }
}
