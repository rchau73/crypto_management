//! Admin-only user management endpoints.

use crate::api::AppState;
use crate::api::auth::CurrentUser;
use crate::api::error::ApiResult;
use crate::domain::models::{Role, User};
use crate::usecases::user_service::{CreateUser, UpdateUser};
use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

pub async fn list(
    current: CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<User>>> {
    current.require(Role::Admin)?;
    Ok(Json(state.users.list().await?))
}

#[derive(Deserialize)]
pub struct CreateUserPayload {
    username: String,
    password: String,
    role: String,
    email: String,
    phone: Option<String>,
}

pub async fn create(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> ApiResult<Json<User>> {
    current.require(Role::Admin)?;
    let user = state
        .users
        .create(CreateUser {
            username: &payload.username,
            password: &payload.password,
            role: &payload.role,
            email: &payload.email,
            phone: payload.phone.as_deref(),
        })
        .await?;
    info!(created_username = %user.username, role = %user.role, by_user_id = current.user_id, "Admin created a user");
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct UpdateUserPayload {
    role: Option<String>,
    password: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

pub async fn update(
    current: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserPayload>,
) -> ApiResult<Json<Value>> {
    current.require(Role::Admin)?;
    state
        .users
        .update(
            id,
            UpdateUser {
                role: payload.role.as_deref(),
                password: payload.password.as_deref(),
                email: payload.email.as_deref(),
                phone: payload.phone.as_deref(),
            },
        )
        .await?;
    info!(
        target_user_id = id,
        by_user_id = current.user_id,
        "Admin updated a user"
    );
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete(
    current: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Value>> {
    current.require(Role::Admin)?;
    state.users.delete(current.user_id, id).await?;
    info!(
        target_user_id = id,
        by_user_id = current.user_id,
        "Admin deleted a user"
    );
    Ok(Json(json!({ "ok": true })))
}
