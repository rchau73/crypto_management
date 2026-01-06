use infra::sqlite::SqliteRepo;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio;
mod infra;
use axum::http::StatusCode;
use axum::{Router, response::Json, routing::get};
use dotenv::dotenv;
use serde_json::json;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

mod api_client;
mod csv_store;
use crate::api_client::{CryptoProvider, ReqwestCryptoProvider};
use crate::domain::repository::HistoryRepo;
mod usecases;
use usecases::allocations_service::AllocationsService;
use usecases::history_service::HistoryService;
mod domain;
use axum::extract::State as AxumState;
use axum::extract::State;
// CSV history module kept for legacy utilities (no fallback used)
// mod csv_history; // legacy CSV helpers removed from runtime flows
use axum::extract::Query;
use chrono::Utc;
use serde::Deserialize as SerdeDeserialize;

#[derive(Clone)]
struct AppState {
    #[allow(dead_code)]
    provider: Arc<dyn CryptoProvider>,
    // DB-backed repo for history and wallet ledger
    history_repo: std::sync::Arc<crate::infra::sqlite::repo::SqliteRepo>,
}

// Define the structure of the API response
#[derive(Deserialize, Serialize, Debug, Clone)]
struct ApiResponse {
    status: ApiStatus,
    data: Vec<CryptoData>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ApiStatus {
    timestamp: String,
    error_code: i32,
    error_message: Option<String>,
    credit_count: i32,
    notice: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct CryptoData {
    id: u32,
    name: String,
    symbol: String,
    cmc_rank: u32,
    tvl_ratio: Option<f64>,
    tvl_usd: Option<f64>,
    quote: QuoteData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct QuoteData {
    #[serde(rename = "USD")]
    usd: PriceInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct PriceInfo {
    price: f64,
    volume_24h: f64,
    percent_change_24h: f64,
    percent_change_7d: f64,
    market_cap: f64,
    #[serde(rename = "fully_diluted_market_cap")]
    fdv: f64,
    tvl: Option<f64>,
}

// Read wallet allocations from CSV

#[tracing::instrument(skip(state))]
async fn api_allocations(
    State(state): AxumState<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    dotenv().ok();
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

    // Use AllocationsService to fetch cryptos, read barca targets, compute allocations and persist allocation record
    let alloc_svc = AllocationsService::new(state.provider.clone(), state.history_repo.clone());
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

    // `result` already computed by AllocationsService; keep using it here
    // Persist historical snapshots (append) using configured paths
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
    // compute total
    let total_value = per_asset
        .iter()
        .map(|a| a.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .sum::<f64>();
    // Persist snapshots into DB via use-case/service (DB-only; CSV legacy persistence removed)
    let history_svc =
        crate::usecases::history_service::HistoryService::new(state.history_repo.clone());
    history_svc
        .persist_snapshots(ts, &per_asset, &per_group, &per_barca, total_value)
        .await;

    // Return computed allocations (no CSV debug fields)
    Ok(Json(result))
}

#[derive(SerdeDeserialize)]
struct HistoryQuery {
    level: Option<String>,
}

async fn api_history(
    State(state): AxumState<AppState>,
    Query(q): Query<HistoryQuery>,
) -> Json<serde_json::Value> {
    let level = q.level.unwrap_or_else(|| "totals".to_string());
    let svc = HistoryService::new(state.history_repo.clone());
    match svc.fetch_history(&level).await {
        Ok(v) => Json(v),
        Err(e) => {
            error!(error = %e, "DB history fetch failed");
            // DB-only mode: return error JSON
            Json(json!({"error": format!("Failed to fetch history from DB: {}", e)}))
        }
    }
}

#[derive(serde::Deserialize)]
struct ImportPayload {
    path: Option<String>,
}

async fn import_wallets_handler(
    State(state): AxumState<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ImportPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let path = payload
        .path
        .unwrap_or_else(|| "wallet_allocations.csv".to_string());
    let svc = HistoryService::new(state.history_repo.clone());
    match svc.import_wallet_allocations_from_path(&path).await {
        Ok(count) => Ok(Json(json!({"imported": count}))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed import: {}", e)})),
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    // DB pool and repo
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/crypto.db".to_string());
    let pool = SqlitePool::connect(&database_url)
        .await
        .expect("failed to connect to db");

    // Run migrations (if any) from ./migrations
    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        error!(error = %e, "Failed to run migrations");
    } else {
        info!("Migrations applied");
    }

    let history_repo = std::sync::Arc::new(SqliteRepo::new(pool.clone()));
    ensure_wallet_allocations_seeded(history_repo.clone()).await;

    let backend_repo = history_repo.clone();
    let backend = async move {
        // initialize provider and app state
        let provider = Arc::new(ReqwestCryptoProvider::new());
        let app_state = AppState {
            provider: provider.clone(),
            history_repo: backend_repo.clone(),
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
    };

    tokio::join!(backend);

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
