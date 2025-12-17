use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use tokio;
use dotenv::dotenv;
use std::error::Error;
use std::collections::HashMap;
use std::sync::Arc;
use axum::{
    routing::get,
    Router,
    response::Json,
};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::cors::{self, Any, CorsLayer};
use axum::http::StatusCode;
use tracing::{info, debug, error, warn};

mod service;
use crate::service::compute_allocations;
mod csv_store;
mod api_client;
use crate::api_client::{CryptoProvider, ReqwestCryptoProvider};
use axum::extract::State as AxumState;
use axum::extract::State;
mod csv_history;
use crate::csv_history::{append_asset_snapshot, append_barca_snapshot, append_totals_snapshot, read_history_csv};
use chrono::Utc;
use axum::extract::Query;
use serde::Deserialize as SerdeDeserialize;
use std::path::PathBuf;

#[derive(Clone)]
struct AppState {
    #[allow(dead_code)]
    provider: Arc<dyn CryptoProvider>,
    history_assets: String,
    history_barca: String,
    history_totals: String,
}
use crate::csv_store::AllocationStore;

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

#[derive(Serialize)]
struct CryptoCsv {
    symbol: String,
    id: u32,
    name: String,
    tvl_ratio: Option<f64>,
    tvl_usd: Option<f64>,
    price: f64,
    volume_24h: f64,
    percent_change_24h: f64,
    percent_change_7d: f64,
    market_cap: f64,
    fdv: f64,
    tvl: Option<f64>,
}

// Struct for wallet allocations with group
#[derive(Debug, Deserialize)]
struct WalletAllocation {
    symbol: String,
    group: String,
    barca: String, // <-- this must match your CSV!
    target_percent: f64,
    current_quantity: f64,
}

#[derive(Debug, Deserialize)]
struct BarcaAllocation {
    market: String,
    group: String,
    target_percent: f64,
}

#[derive(Deserialize)]
struct Allocation {
    symbol: String,
    group: String,
    barca: String,
    target_percent: f64,
    current_quantity: f64,
    #[serde(default)]
    comments: Option<String>, // This will be ignored in all calculations
}


// Function to fetch cryptocurrency data
async fn fetch_crypto_data(api_key: &str) -> Result<Vec<CryptoData>, ReqwestError> {
    let client = Client::new();
    let url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";

    let mut params = std::collections::HashMap::new();
    params.insert("limit", "1000");

    let response = client.get(url)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await?;

    // Deserialize JSON into our struct
    let parsed_response: ApiResponse = response.json().await?;
    info!(status = ?parsed_response.status, fetched = parsed_response.data.len());
    debug!(data = ?parsed_response.data);
    Ok(parsed_response.data)
}

// Read wallet allocations from CSV
 
 
#[tracing::instrument(skip(state))]
async fn api_allocations(State(state): AxumState<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    dotenv().ok();
    let api_key = match std::env::var("API_KEY") {
        Ok(k) => k,
        Err(_) => {
            error!("Missing API_KEY environment variable");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Missing API_KEY"}))));
        }
    };

    let current_market = std::env::var("CURRENT_MARKET").unwrap_or_else(|_| "BullMarket".to_string());

    let store = crate::csv_store::FileCsvStore;
    let allocations = match store.read_wallet_allocations("wallet_allocations.csv") {
        Ok(a) => a,
        Err(e) => {
            error!(error = %e, "Failed reading wallet allocations");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed reading wallet allocations: {}", e)}))));
        }
    };

    let cryptos = match state.provider.fetch_latest(&api_key).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed fetching crypto data");
            return Err((StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Failed fetching crypto data: {}", e)}))));
        }
    };

    let barca_targets = match store.read_barca_allocations("wallet_barca.csv", &current_market) {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "Failed reading barca allocations");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed reading barca allocations: {}", e)}))));
        }
    };

    let result = compute_allocations(&allocations, &cryptos, &barca_targets);
    // Persist historical snapshots (append) using configured paths
    let ts = Utc::now();
    let per_asset = result.get("per_asset").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let per_barca_actual = result.get("per_barca_actual").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    // compute total
    let total_value = per_asset.iter().map(|a| a.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0)).sum::<f64>();
    // Attempt to persist snapshots and log results for diagnostics
    match append_asset_snapshot(&ts, &per_asset, &state.history_assets) {
        Ok(_) => info!(path = %state.history_assets, "Appended asset snapshot"),
        Err(e) => error!(path = %state.history_assets, error = %e, "Failed to append asset snapshot"),
    }
    match append_barca_snapshot(&ts, &per_barca_actual, &state.history_barca) {
        Ok(_) => info!(path = %state.history_barca, "Appended barca snapshot"),
        Err(e) => error!(path = %state.history_barca, error = %e, "Failed to append barca snapshot"),
    }
    match append_totals_snapshot(&ts, total_value, &state.history_totals) {
        Ok(_) => info!(path = %state.history_totals, "Appended totals snapshot"),
        Err(e) => error!(path = %state.history_totals, error = %e, "Failed to append totals snapshot"),
    }
    // Provide diagnostic debug info with absolute paths and append status
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let abs_assets = cwd.join(&state.history_assets).to_string_lossy().to_string();
    let abs_barca = cwd.join(&state.history_barca).to_string_lossy().to_string();
    let abs_totals = cwd.join(&state.history_totals).to_string_lossy().to_string();

    // Re-run append calls but capture results for response (we already logged above; re-check return values)
    let assets_status = match append_asset_snapshot(&ts, &per_asset, &state.history_assets) {
        Ok(_) => (true, None::<String>),
        Err(e) => (false, Some(format!("{}", e))),
    };
    let barca_status = match append_barca_snapshot(&ts, &per_barca_actual, &state.history_barca) {
        Ok(_) => (true, None::<String>),
        Err(e) => (false, Some(format!("{}", e))),
    };
    let totals_status = match append_totals_snapshot(&ts, total_value, &state.history_totals) {
        Ok(_) => (true, None::<String>),
        Err(e) => (false, Some(format!("{}", e))),
    };

    let mut resp = result;
    let debug = json!({
        "cwd": cwd.to_string_lossy().to_string(),
        "assets": {"path": state.history_assets, "abs_path": abs_assets, "ok": assets_status.0, "error": assets_status.1},
        "barca": {"path": state.history_barca, "abs_path": abs_barca, "ok": barca_status.0, "error": barca_status.1},
        "totals": {"path": state.history_totals, "abs_path": abs_totals, "ok": totals_status.0, "error": totals_status.1},
    });
    if let serde_json::Value::Object(ref mut m) = resp {
        m.insert("debug".to_string(), debug);
    }
    Ok(Json(resp))
}

#[derive(SerdeDeserialize)]
struct HistoryQuery {
    level: Option<String>,
}

async fn api_history(Query(q): Query<HistoryQuery>) -> Json<serde_json::Value> {
    let level = q.level.unwrap_or_else(|| "totals".to_string());
    let out = match level.as_str() {
        "assets" => read_history_csv("history_assets.csv").unwrap_or_default(),
        "barca" => read_history_csv("history_barca.csv").unwrap_or_default(),
        _ => read_history_csv("history_totals.csv").unwrap_or_default(),
    };
    Json(json!({"level": level, "rows": out}))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    let backend = async {
            // initialize provider and app state
            let provider = Arc::new(ReqwestCryptoProvider::new());
            let app_state = AppState {
                provider: provider.clone(),
                history_assets: "history_assets.csv".to_string(),
                history_barca: "history_barca.csv".to_string(),
                history_totals: "history_totals.csv".to_string(),
            };

            let app = Router::new()
        .route("/api/allocations", get(api_allocations))
        .route("/api/history", get(api_history))
        .with_state(app_state)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any));

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
    error!("Failed to bind to any port in range {}..{}", port, port + max_attempts - 1);
}

