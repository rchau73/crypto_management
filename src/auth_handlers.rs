//! Axum glue for authentication and user management: the `CurrentUser`
//! extractor (verifies the access-token cookie on every protected request)
//! and the `/api/auth/*` + `/api/admin/users*` handlers. Kept out of
//! `main.rs` so that file stays composition-root-sized as this grows.

use crate::AppState;
use crate::domain::models::Role;
use crate::domain::repository::{RefreshTokenRepo, UserRepo};
use crate::usecases::auth_service::{
    ACCESS_TOKEN_TTL_MINUTES, AuthService, REFRESH_TOKEN_TTL_DAYS, verify_access_token,
};
use crate::usecases::user_service::{UserService, UserServiceError};
use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{info, warn};

type ApiError = (StatusCode, Json<Value>);

fn error_response(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": message.into() })))
}

fn internal_error(e: impl std::fmt::Display) -> ApiError {
    error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn user_service_error_response(e: UserServiceError) -> ApiError {
    match e {
        UserServiceError::InvalidRole(_) | UserServiceError::InvalidEmail(_) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        UserServiceError::UsernameTaken | UserServiceError::EmailTaken => {
            error_response(StatusCode::CONFLICT, e.to_string())
        }
        UserServiceError::Repo(_) => internal_error(e),
    }
}

/// The authenticated caller, extracted and verified from the `access_token`
/// cookie. Adding this as a handler parameter is what makes a route require
/// login — Axum rejects the request with 401 before the handler body runs
/// if the cookie is missing, expired, or has a bad signature.
#[derive(Debug)]
pub struct CurrentUser {
    pub user_id: i64,
    pub role: Role,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get("access_token")
            .map(|c| c.value().to_string())
            .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "Not logged in"))?;

        let claims = verify_access_token(&token, &state.jwt_secret)
            .map_err(|_| error_response(StatusCode::UNAUTHORIZED, "Invalid or expired session"))?;
        let user_id: i64 = claims
            .sub
            .parse()
            .map_err(|_| error_response(StatusCode::UNAUTHORIZED, "Malformed session token"))?;
        let role = Role::parse(&claims.role).ok_or_else(|| {
            error_response(StatusCode::UNAUTHORIZED, "Unknown role in session token")
        })?;

        Ok(CurrentUser { user_id, role })
    }
}

/// Returns 403 unless the caller's role is at least `minimum`.
pub fn require_role(current: &CurrentUser, minimum: Role) -> Result<(), ApiError> {
    if current.role.satisfies(minimum) {
        Ok(())
    } else {
        Err(error_response(
            StatusCode::FORBIDDEN,
            "Insufficient permissions",
        ))
    }
}

fn auth_service(state: &AppState) -> AuthService {
    let users: Arc<dyn UserRepo> = state.history_repo.clone();
    let refresh_tokens: Arc<dyn RefreshTokenRepo> = state.history_repo.clone();
    AuthService::new(users, refresh_tokens, state.jwt_secret.clone())
}

fn cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v == "true")
        .unwrap_or(false)
}

fn build_cookie(
    name: &'static str,
    value: String,
    path: &'static str,
    max_age_seconds: i64,
) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(cookie_secure())
        .same_site(SameSite::Lax)
        .path(path)
        .max_age(time::Duration::seconds(max_age_seconds))
        .build()
}

fn access_cookie(token: String) -> Cookie<'static> {
    build_cookie("access_token", token, "/", ACCESS_TOKEN_TTL_MINUTES * 60)
}

fn refresh_cookie(token: String) -> Cookie<'static> {
    build_cookie(
        "refresh_token",
        token,
        "/api/auth",
        REFRESH_TOKEN_TTL_DAYS * 24 * 60 * 60,
    )
}

#[derive(Deserialize)]
pub struct LoginPayload {
    username: String,
    password: String,
}

#[tracing::instrument(skip(state, payload))]
pub async fn login_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginPayload>,
) -> Result<(CookieJar, Json<Value>), ApiError> {
    match auth_service(&state)
        .login(&payload.username, &payload.password)
        .await
    {
        Ok(result) => {
            info!(username = %payload.username, "Login succeeded");
            let jar = jar
                .add(access_cookie(result.access_token))
                .add(refresh_cookie(result.refresh_token));
            Ok((
                jar,
                Json(
                    json!({ "username": result.user.username, "role": result.user.role, "email": result.user.email }),
                ),
            ))
        }
        Err(e) => {
            warn!(username = %payload.username, error = %e, "Login failed");
            Err(error_response(
                StatusCode::UNAUTHORIZED,
                "Invalid username or password",
            ))
        }
    }
}

#[tracing::instrument(skip(state, jar))]
pub async fn logout_handler(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (CookieJar, Json<Value>) {
    if let Some(cookie) = jar.get("refresh_token") {
        if let Err(e) = auth_service(&state).logout(cookie.value()).await {
            warn!(error = %e, "Failed to revoke refresh token on logout");
        }
    }
    let jar = jar
        .remove(Cookie::from("access_token"))
        .remove(Cookie::from("refresh_token"));
    (jar, Json(json!({ "ok": true })))
}

#[tracing::instrument(skip(state, jar))]
pub async fn refresh_handler(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<Value>), ApiError> {
    let token = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "Not logged in"))?;

    match auth_service(&state).refresh(&token).await {
        Ok(new_access_token) => {
            let jar = jar.add(access_cookie(new_access_token));
            Ok((jar, Json(json!({ "ok": true }))))
        }
        Err(e) => {
            warn!(error = %e, "Refresh failed");
            Err(error_response(
                StatusCode::UNAUTHORIZED,
                "Session expired, please log in again",
            ))
        }
    }
}

pub async fn me_handler(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let user_repo: Arc<dyn UserRepo> = state.history_repo.clone();
    let user = user_repo
        .find_user_by_id(current.user_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "User no longer exists"))?;
    Ok(Json(
        json!({ "username": user.username, "role": user.role, "email": user.email }),
    ))
}

pub async fn list_users_handler(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Admin)?;
    let users = UserService::new(state.history_repo.clone())
        .list()
        .await
        .map_err(internal_error)?;
    Ok(Json(json!(users)))
}

#[derive(Deserialize)]
pub struct CreateUserPayload {
    username: String,
    password: String,
    role: String,
    email: String,
    phone: Option<String>,
}

pub async fn create_user_handler(
    current: CurrentUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Admin)?;
    let user = UserService::new(state.history_repo.clone())
        .create(
            &payload.username,
            &payload.password,
            &payload.role,
            &payload.email,
            payload.phone.as_deref(),
        )
        .await
        .map_err(user_service_error_response)?;
    info!(created_username = %user.username, role = %user.role, by_user_id = current.user_id, "Admin created a user");
    Ok(Json(json!(user)))
}

#[derive(Deserialize)]
pub struct UpdateUserPayload {
    role: Option<String>,
    password: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

pub async fn update_user_handler(
    current: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserPayload>,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Admin)?;
    UserService::new(state.history_repo.clone())
        .update(
            id,
            payload.role.as_deref(),
            payload.password.as_deref(),
            payload.email.as_deref(),
            payload.phone.as_deref(),
        )
        .await
        .map_err(user_service_error_response)?;
    info!(
        target_user_id = id,
        by_user_id = current.user_id,
        "Admin updated a user"
    );
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_user_handler(
    current: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Admin)?;
    if current.user_id == id {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "Cannot delete your own account",
        ));
    }
    UserService::new(state.history_repo.clone())
        .delete(id)
        .await
        .map_err(internal_error)?;
    info!(
        target_user_id = id,
        by_user_id = current.user_id,
        "Admin deleted a user"
    );
    Ok(Json(json!({ "ok": true })))
}
