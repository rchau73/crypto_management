//! Login/logout/session endpoints and the `CurrentUser` extractor that
//! protects every other route.

use crate::api::AppState;
use crate::api::error::{ApiError, ApiResult};
use crate::domain::models::{Role, User};
use crate::usecases::auth_service::{ACCESS_TOKEN_TTL_MINUTES, REFRESH_TOKEN_TTL_DAYS};
use crate::usecases::user_service::UserServiceError;
use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

const ACCESS_COOKIE: &str = "access_token";
const REFRESH_COOKIE: &str = "refresh_token";

/// The logged-in caller. Adding this as a handler parameter is what makes
/// a route require login: the request is rejected with 401 before the
/// handler runs if the access-token cookie is missing, expired or forged.
#[derive(Debug, Clone, Copy)]
pub struct CurrentUser {
    pub user_id: i64,
    pub role: Role,
}

impl CurrentUser {
    /// 403 unless the caller's role is at least `minimum`.
    pub fn require(&self, minimum: Role) -> ApiResult<()> {
        if self.role.satisfies(minimum) {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> ApiResult<Self> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(ACCESS_COOKIE)
            .ok_or_else(|| ApiError::unauthorized("Not logged in"))?;
        let session = state
            .auth
            .authenticate(token.value())
            .map_err(|_| ApiError::unauthorized("Invalid or expired session"))?;
        Ok(CurrentUser {
            user_id: session.user_id,
            role: session.role,
        })
    }
}

fn cookie(
    name: &'static str,
    value: String,
    path: &'static str,
    max_age_seconds: i64,
    secure: bool,
) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path(path)
        .max_age(time::Duration::seconds(max_age_seconds))
        .build()
}

fn access_cookie(token: String, secure: bool) -> Cookie<'static> {
    cookie(
        ACCESS_COOKIE,
        token,
        "/",
        ACCESS_TOKEN_TTL_MINUTES * 60,
        secure,
    )
}

fn refresh_cookie(token: String, secure: bool) -> Cookie<'static> {
    // Only sent to /api/auth/*, where it is needed.
    cookie(
        REFRESH_COOKIE,
        token,
        "/api/auth",
        REFRESH_TOKEN_TTL_DAYS * 24 * 60 * 60,
        secure,
    )
}

fn profile(user: &User) -> Value {
    json!({ "username": user.username, "role": user.role, "email": user.email })
}

#[derive(Deserialize)]
pub struct LoginPayload {
    username: String,
    password: String,
}

#[tracing::instrument(skip_all, fields(username = %payload.username))]
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginPayload>,
) -> ApiResult<(CookieJar, Json<Value>)> {
    let result = state
        .auth
        .login(&payload.username, &payload.password)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "Login failed");
            ApiError::unauthorized("Invalid username or password")
        })?;
    info!("Login succeeded");
    let jar = jar
        .add(access_cookie(result.access_token, state.cookie_secure))
        .add(refresh_cookie(result.refresh_token, state.cookie_secure));
    Ok((jar, Json(profile(&result.user))))
}

#[tracing::instrument(skip_all)]
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> (CookieJar, Json<Value>) {
    if let Some(token) = jar.get(REFRESH_COOKIE) {
        if let Err(e) = state.auth.logout(token.value()).await {
            tracing::warn!(error = %e, "Failed to revoke refresh token on logout");
        }
    }
    let jar = jar
        .remove(Cookie::build(ACCESS_COOKIE).path("/"))
        .remove(Cookie::build(REFRESH_COOKIE).path("/api/auth"));
    (jar, Json(json!({ "ok": true })))
}

#[tracing::instrument(skip_all)]
pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<Value>)> {
    let token = jar
        .get(REFRESH_COOKIE)
        .ok_or_else(|| ApiError::unauthorized("Not logged in"))?;
    let access_token = state.auth.refresh(token.value()).await.map_err(|e| {
        tracing::warn!(error = %e, "Refresh failed");
        ApiError::unauthorized("Session expired, please log in again")
    })?;
    let jar = jar.add(access_cookie(access_token, state.cookie_secure));
    Ok((jar, Json(json!({ "ok": true }))))
}

pub async fn me(current: CurrentUser, State(state): State<AppState>) -> ApiResult<Json<Value>> {
    match state.users.find(current.user_id).await {
        Ok(user) => Ok(Json(profile(&user))),
        Err(UserServiceError::NotFound) => Err(ApiError::unauthorized("User no longer exists")),
        Err(e) => Err(e.into()),
    }
}
