//! The portfolio list (account switcher), creating/renaming portfolios, and
//! the `PortfolioScope` extractor that guards every
//! `/api/portfolios/{id}/...` route.

use crate::api::AppState;
use crate::api::auth::CurrentUser;
use crate::api::error::{ApiError, ApiResult};
use crate::domain::models::{PortfolioAccess, PortfolioId, PortfolioSummary, Role};
use axum::Json;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::request::Parts;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

/// A logged-in caller who is a member of the portfolio in the URL. Taking
/// this as a handler parameter is what scopes the handler to one portfolio:
/// the request is rejected before the handler runs if the caller isn't
/// logged in (401) or isn't a member (404 — as if the portfolio didn't
/// exist). Handlers then pass `scope.portfolio` to the service.
#[derive(Debug, Clone, Copy)]
pub struct PortfolioScope {
    pub user: CurrentUser,
    pub portfolio: PortfolioId,
    pub access: PortfolioAccess,
}

impl PortfolioScope {
    /// 403 unless the caller may change this portfolio: a manager (or
    /// admin) role *and* `manage` access to it.
    pub fn require_manage(&self) -> ApiResult<()> {
        self.user.require(Role::Manager)?;
        if self.access == PortfolioAccess::Manage {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
}

impl FromRequestParts<AppState> for PortfolioScope {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> ApiResult<Self> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path(id) = Path::<i64>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::not_found("portfolio not found"))?;
        let portfolio = PortfolioId(id);
        let access = state.portfolios.access(portfolio, user.user_id).await?;
        Ok(PortfolioScope {
            user,
            portfolio,
            access,
        })
    }
}

/// The portfolios the caller can open (the account switcher).
pub async fn list(
    current: CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<PortfolioSummary>>> {
    Ok(Json(state.portfolios.list_for_user(current.user_id).await?))
}

#[derive(Deserialize)]
pub struct CreatePortfolioPayload {
    name: String,
    /// Start with a copy of this portfolio's BARCA targets.
    copy_barca_from: Option<PortfolioId>,
}

#[tracing::instrument(skip_all, fields(user_id = current.user_id))]
pub async fn create(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<CreatePortfolioPayload>,
) -> ApiResult<Json<PortfolioSummary>> {
    current.require(Role::Manager)?;
    let created = state
        .portfolios
        .create(&payload.name, current.user_id, payload.copy_barca_from)
        .await?;
    Ok(Json(created))
}

#[derive(Deserialize)]
pub struct RenamePortfolioPayload {
    name: String,
}

#[tracing::instrument(skip_all, fields(user_id = scope.user.user_id, portfolio_id = %scope.portfolio))]
pub async fn rename(
    scope: PortfolioScope,
    State(state): State<AppState>,
    Json(payload): Json<RenamePortfolioPayload>,
) -> ApiResult<Json<Value>> {
    scope.require_manage()?;
    state
        .portfolios
        .rename(scope.portfolio, scope.user.user_id, &payload.name)
        .await?;
    info!("Renamed portfolio");
    Ok(Json(json!({ "ok": true })))
}
