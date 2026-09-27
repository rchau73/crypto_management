mod domain;
mod infra;
mod usecases;

mod csv_store;

use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Router, response::Json, routing::get};
use chrono::Utc;
use dotenv::dotenv;
use serde::Deserialize;
use serde_json::json;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

use crate::csv_store::{AllocationStore, FileCsvStore};
use crate::domain::market_data::CryptoProvider;
use crate::domain::repository::HistoryRepo;
use crate::infra::coinmarketcap::ReqwestCryptoProvider;
use crate::infra::sqlite::SqliteRepo;
use sqlx::SqlitePool;
use usecases::allocations_service::AllocationsService;
use usecases::history_service::HistoryService;

/// Everything an Axum handler needs. Composed once in `main` and shared
/// (cheaply, via `Arc`/`Clone`) across every request.
#[derive(Clone)]
struct AppState {
    provider: Arc<dyn CryptoProvider>,
    history_repo: Arc<SqliteRepo>,
    allocation_store: Arc<dyn AllocationStore>,
}

#[tracing::instrument(skip(state))]
async fn api_allocations(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let api_key = match std::env::var("API_KEY") {
        Ok(k) => k,
        Err(_) => {
            error!("Missing API_KEY environment variable");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Missing API_KEY"})),
            ));
        }
    };

    let current_market =
        std::env::var("CURRENT_MARKET").unwrap_or_else(|_| "BullMarket".to_string());

    let alloc_svc = AllocationsService::new(
        state.provider.clone(),
        state.history_repo.clone(),
        state.allocation_store.clone(),
    );
    let result = match alloc_svc
        .compute_and_record(&api_key, &current_market)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, "Failed computing allocations");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed computing allocations: {}", e)})),
            ));
        }
    };

    // Persist historical snapshots (append) into SQLite.
    let ts = Utc::now();
    let per_asset = result
        .get("per_asset")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let per_group = result
        .get("per_group")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let per_barca = result
        .get("per_barca")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let total_value = per_asset
        .iter()
        .map(|a| a.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .sum::<f64>();

    let history_svc = HistoryService::new(state.history_repo.clone());
    history_svc
        .persist_snapshots(ts, &per_asset, &per_group, &per_barca, total_value)
        .await;

    info!(
        assets = per_asset.len(),
        groups = per_group.len(),
        total_value,
        "Computed and persisted allocation snapshot"
    );

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct HistoryQuery {
    level: Option<String>,
}

#[tracing::instrument(skip(state))]
async fn api_history(
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> Json<serde_json::Value> {
    let level = q.level.unwrap_or_else(|| "totals".to_string());
    let svc = HistoryService::new(state.history_repo.clone());
    match svc.fetch_history(&level).await {
        Ok(v) => Json(v),
        Err(e) => {
            error!(error = %e, level = %level, "DB history fetch failed");
            Json(json!({"error": format!("Failed to fetch history from DB: {}", e)}))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ImportPayload {
    path: Option<String>,
}

#[tracing::instrument(skip(state))]
async fn import_wallets_handler(
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ImportPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let path = payload
        .path
        .unwrap_or_else(|| "wallet_allocations.csv".to_string());
    let svc = HistoryService::new(state.history_repo.clone());
    match svc.import_wallet_allocations_from_path(&path).await {
        Ok(count) => {
            info!(path = %path, imported = count, "Imported wallet allocations");
            Ok(Json(json!({"imported": count})))
        }
        Err(e) => {
            error!(error = %e, path = %path, "Failed to import wallet allocations");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed import: {}", e)})),
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/crypto.db".to_string());
    let pool = SqlitePool::connect(&database_url)
        .await
        .expect("failed to connect to db");

    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        error!(error = %e, "Failed to run migrations");
    } else {
        info!("Migrations applied");
    }

    let history_repo = Arc::new(SqliteRepo::new(pool.clone()));
    ensure_wallet_allocations_seeded(history_repo.clone()).await;

    let provider = Arc::new(ReqwestCryptoProvider::new());
    let allocation_store = Arc::new(FileCsvStore);
    let app_state = AppState {
        provider,
        history_repo,
        allocation_store,
    };

    let app = Router::new()
        .route("/api/allocations", get(api_allocations))
        .route("/api/history", get(api_history))
        .route(
            "/api/import_wallets",
            axum::routing::post(import_wallets_handler),
        )
        .with_state(app_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    serve(app, 3001).await;

    Ok(())
}

async fn serve(app: Router, port: u16) {
    // Try to bind to the requested port; if it's in use, try a few subsequent ports.
    let max_attempts = 10;
    for offset in 0..max_attempts {
        let try_port = port + offset;
        let addr = SocketAddr::from(([127, 0, 0, 1], try_port));
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                println!("Listening on {}", addr);
                if let Err(e) = axum::serve(listener, app).await {
                    error!(error = %e, "Server failed while serving");
                }
                return;
            }
            Err(e) => {
                warn!(port = try_port, error = %e, "Port unavailable, trying next");
            }
        }
    }
    error!(
        "Failed to bind to any port in range {}..{}",
        port,
        port + max_attempts - 1
    );
}

async fn ensure_wallet_allocations_seeded(history_repo: Arc<SqliteRepo>) {
    match history_repo.fetch_current_wallet_allocations().await {
        Ok(rows) if rows.is_empty() => {
            let path = std::env::var("WALLET_ALLOCATIONS_PATH")
                .unwrap_or_else(|_| "wallet_allocations.csv".to_string());
            let history_svc = HistoryService::new(history_repo.clone());
            match history_svc.import_wallet_allocations_from_path(&path).await {
                Ok(imported) => info!(
                    path = %path,
                    imported,
                    "Bootstrapped wallet_allocations from CSV"
                ),
                Err(e) => warn!(
                    path = %path,
                    error = %e,
                    "Failed to bootstrap wallet allocations from CSV"
                ),
            }
        }
        Ok(_) => {}
        Err(e) => warn!(
            error = %e,
            "Unable to inspect wallet allocations before bootstrap"
        ),
    }
}

#[cfg(test)]
mod tests;
