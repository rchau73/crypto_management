//! Axum glue for seeding `wallet_allocations` from a CSV — either a path on
//! the server's own filesystem (useful for local dev, where you can just
//! edit the file in the repo) or an actual file upload (the only option
//! once the server is a container/serverless deploy with no filesystem you
//! can reach or edit directly). Both funnel into the same
//! `HistoryService::import_wallet_allocations_from_path`. Kept out of
//! `main.rs` for the same reason as `auth_handlers.rs`/`targets_handlers.rs`
//! — that file stays composition-root-sized.

use crate::AppState;
use crate::auth_handlers::{CurrentUser, require_role};
use crate::domain::models::Role;
use crate::usecases::history_service::{HistoryService, ImportError};
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{error, info, warn};

type ApiError = (StatusCode, Json<Value>);

fn error_response(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": message.into() })))
}

/// A malformed or non-CSV upload is the caller's mistake, not a server
/// fault — `ImportError::is_client_error` tells apart "you gave us
/// something that isn't a wallet CSV" (400) from a real backend problem
/// (500).
fn import_error_response(e: &ImportError) -> ApiError {
    if e.is_client_error() {
        return error_response(StatusCode::BAD_REQUEST, format!("Invalid CSV file: {e}"));
    }
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed import: {e}"),
    )
}

#[derive(Debug, Deserialize)]
pub struct ImportPayload {
    path: Option<String>,
}

#[tracing::instrument(skip(state))]
pub async fn import_wallets_handler(
    current: CurrentUser,
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ImportPayload>,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Manager)?;
    let path = payload
        .path
        .unwrap_or_else(|| "wallet_allocations.csv".to_string());
    let svc = HistoryService::new(state.history_repo.clone(), state.history_repo.clone());
    match svc.import_wallet_allocations_from_path(&path).await {
        Ok(count) => {
            info!(path = %path, imported = count, "Imported wallet allocations");
            Ok(Json(json!({"imported": count})))
        }
        Err(e) => {
            let response = import_error_response(&e);
            if response.0 == StatusCode::BAD_REQUEST {
                warn!(error = %e, path = %path, "Rejected an invalid wallet allocations CSV");
            } else {
                error!(error = %e, path = %path, "Failed to import wallet allocations");
            }
            Err(response)
        }
    }
}

/// A temp file that deletes itself on drop, success or failure alike — the
/// uploaded CSV only needs to exist for the single
/// `import_wallet_allocations_from_path` call, and nothing about this
/// endpoint should leave stray files behind on repeated use.
struct TempCsvFile(std::path::PathBuf);

impl Drop for TempCsvFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[tracing::instrument(skip(state, multipart))]
pub async fn import_wallets_upload_handler(
    current: CurrentUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    require_role(&current, Role::Manager)?;

    let mut csv_bytes: Option<Vec<u8>> = None;
    loop {
        let field = multipart
            .next_field()
            .await
            .map_err(|e| error_response(StatusCode::BAD_REQUEST, format!("Bad upload: {e}")))?;
        let Some(field) = field else { break };
        if field.name() == Some("file") {
            // Grabbed before `.bytes()` consumes the field — the browser's
            // own filename/type is a cheap first filter that catches an
            // obviously-wrong file (a .docx, .pptx, .png, ...) before we
            // even look at its content.
            let file_name = field.file_name().map(str::to_lowercase);
            if let Some(name) = &file_name {
                if !(name.ends_with(".csv") || name.ends_with(".txt")) {
                    return Err(error_response(
                        StatusCode::BAD_REQUEST,
                        format!("Expected a .csv file, got \"{name}\""),
                    ));
                }
            }
            let bytes = field
                .bytes()
                .await
                .map_err(|e| error_response(StatusCode::BAD_REQUEST, format!("Bad upload: {e}")))?;
            csv_bytes = Some(bytes.to_vec());
            break;
        }
    }
    let Some(csv_bytes) = csv_bytes else {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "Missing \"file\" field in upload",
        ));
    };

    // A second, content-based check regardless of what the filename claimed
    // — catches a renamed binary file, and gives a much clearer error than
    // whatever the CSV parser would eventually fail with. Word/PowerPoint
    // (both ZIP- and OLE-based), images, PDFs etc. all contain null bytes
    // or invalid UTF-8 almost immediately; a real CSV never does.
    if csv_bytes.contains(&0u8) || std::str::from_utf8(&csv_bytes).is_err() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "File does not look like a text/CSV file (binary content detected)",
        ));
    }

    let tmp_path = std::env::temp_dir().join(format!(
        "wallet_upload_{}_{}.csv",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::write(&tmp_path, &csv_bytes).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to stage upload: {e}"),
        )
    })?;
    let tmp_file = TempCsvFile(tmp_path.clone());

    let svc = HistoryService::new(state.history_repo.clone(), state.history_repo.clone());
    let path_str = tmp_file.0.to_string_lossy().to_string();
    match svc.import_wallet_allocations_from_path(&path_str).await {
        Ok(count) => {
            info!(
                imported = count,
                bytes = csv_bytes.len(),
                by_user_id = current.user_id,
                "Imported wallet allocations from an uploaded CSV"
            );
            Ok(Json(json!({"imported": count})))
        }
        Err(e) => {
            let response = import_error_response(&e);
            if response.0 == StatusCode::BAD_REQUEST {
                warn!(error = %e, by_user_id = current.user_id, "Rejected an invalid uploaded wallet allocations CSV");
            } else {
                error!(error = %e, by_user_id = current.user_id, "Failed to import uploaded wallet allocations CSV");
            }
            Err(response)
        }
    }
}
