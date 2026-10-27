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

/// Names of the two auth cookies. Browsers share cookies between every
/// port on the same host, so each app gets its own prefix (`COOKIE_PREFIX`);
/// otherwise logging into another local app with the same cookie names
/// overwrites — and so ends — this app's session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieNames {
    pub access: String,
    pub refresh: String,
}

impl CookieNames {
    /// `"cm"` -> `cm_access_token` / `cm_refresh_token`; an empty prefix
    /// keeps the plain `access_token` / `refresh_token` names.
    pub fn new(prefix: &str) -> Self {
        let name = |base: &str| {
            if prefix.is_empty() {
                base.to_string()
            } else {
                format!("{prefix}_{base}")
            }
        };
        Self {
            access: name("access_token"),
            refresh: name("refresh_token"),
        }
    }
}

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
            .get(&state.cookie_names.access)
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
    name: String,
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

fn access_cookie(state: &AppState, token: String) -> Cookie<'static> {
    cookie(
        state.cookie_names.access.clone(),
        token,
        "/",
        ACCESS_TOKEN_TTL_MINUTES * 60,
        state.cookie_secure,
    )
}

fn refresh_cookie(state: &AppState, token: String) -> Cookie<'static> {
    // Only sent to /api/auth/*, where it is needed.
    cookie(
        state.cookie_names.refresh.clone(),
        token,
        "/api/auth",
        REFRESH_TOKEN_TTL_DAYS * 24 * 60 * 60,
        state.cookie_secure,
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
        .add(access_cookie(&state, result.access_token))
        .add(refresh_cookie(&state, result.refresh_token));
    Ok((jar, Json(profile(&result.user))))
}

#[tracing::instrument(skip_all)]
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> (CookieJar, Json<Value>) {
    let names = &state.cookie_names;
    if let Some(token) = jar.get(&names.refresh) {
        if let Err(e) = state.auth.logout(token.value()).await {
            tracing::warn!(error = %e, "Failed to revoke refresh token on logout");
        }
    }
    let jar = jar
        .remove(Cookie::build(names.access.clone()).path("/"))
        .remove(Cookie::build(names.refresh.clone()).path("/api/auth"));
    (jar, Json(json!({ "ok": true })))
}

#[tracing::instrument(skip_all)]
pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<Value>)> {
    let token = jar
        .get(&state.cookie_names.refresh)
        .ok_or_else(|| ApiError::unauthorized("Not logged in"))?;
    let access_token = state.auth.refresh(token.value()).await.map_err(|e| {
        tracing::warn!(error = %e, "Refresh failed");
        ApiError::unauthorized("Session expired, please log in again")
    })?;
    let jar = jar.add(access_cookie(&state, access_token));
    Ok((jar, Json(json!({ "ok": true }))))
}

pub async fn me(current: CurrentUser, State(state): State<AppState>) -> ApiResult<Json<Value>> {
    match state.users.find(current.user_id).await {
        Ok(user) => Ok(Json(profile(&user))),
        Err(UserServiceError::NotFound) => Err(ApiError::unauthorized("User no longer exists")),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_names_are_prefixed() {
        let names = CookieNames::new("cm");
        assert_eq!(names.access, "cm_access_token");
        assert_eq!(names.refresh, "cm_refresh_token");
    }

    #[test]
    fn an_empty_prefix_keeps_the_plain_names() {
        let names = CookieNames::new("");
        assert_eq!(names.access, "access_token");
        assert_eq!(names.refresh, "refresh_token");
    }
}
