//! One error type for every handler. Service errors convert into it with
//! `?`, and it renders as `{"error": "..."}` with the right status code.

use crate::usecases::allocations_service::AllocationsError;
use crate::usecases::targets_service::TargetsError;
use crate::usecases::user_service::UserServiceError;
use crate::usecases::wallet_import::ImportError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::fmt::Display;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    /// Shown to the user.
    message: String,
    /// Logged only — may contain internals (SQL errors, file paths...).
    detail: Option<String>,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            detail: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "Insufficient permissions")
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    /// A server-side failure. The real error is logged; the user sees a
    /// generic message.
    pub fn internal(error: impl Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal server error".to_string(),
            detail: Some(error.to_string()),
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let detail = self.detail.as_deref().unwrap_or(&self.message);
        if self.status.is_server_error() {
            tracing::error!(status = %self.status, error = %detail, "Request failed");
        } else {
            tracing::warn!(status = %self.status, error = %detail, "Request rejected");
        }
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

impl From<TargetsError> for ApiError {
    fn from(e: TargetsError) -> Self {
        match e {
            TargetsError::Invalid(message) => ApiError::bad_request(message),
            TargetsError::NotFound(message) => ApiError::not_found(message),
            TargetsError::Repo(e) => ApiError::internal(e),
        }
    }
}

impl From<UserServiceError> for ApiError {
    fn from(e: UserServiceError) -> Self {
        let status = match &e {
            UserServiceError::InvalidRole(_)
            | UserServiceError::InvalidEmail(_)
            | UserServiceError::CannotDeleteSelf => StatusCode::BAD_REQUEST,
            UserServiceError::UsernameTaken
            | UserServiceError::EmailTaken
            | UserServiceError::LastAdmin => StatusCode::CONFLICT,
            UserServiceError::NotFound => StatusCode::NOT_FOUND,
            UserServiceError::Repo(_) => return ApiError::internal(e),
        };
        ApiError::new(status, e.to_string())
    }
}

impl From<ImportError> for ApiError {
    fn from(e: ImportError) -> Self {
        if e.is_client_error() {
            ApiError::bad_request(format!("Invalid CSV file: {e}"))
        } else {
            ApiError::internal(e)
        }
    }
}

impl From<AllocationsError> for ApiError {
    fn from(e: AllocationsError) -> Self {
        match e {
            // The upstream price API failed, not us: 502, and the reason
            // (bad key, rate limit...) is useful to the user.
            AllocationsError::CryptoPrices(_) => {
                ApiError::new(StatusCode::BAD_GATEWAY, e.to_string())
            }
            AllocationsError::Repo(_) => ApiError::internal(e),
        }
    }
}
