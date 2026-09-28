mod domain;
mod infra;
mod usecases;

mod auth_handlers;
mod targets_handlers;

use axum::extract::Query;
use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::{Router, response::Json, routing::get};
use chrono::Utc;
use dotenv::dotenv;
use serde::Deserialize;
use serde_json::json;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

use crate::auth_handlers::{
    CurrentUser, create_user_handler, delete_user_handler, list_users_handler, login_handler,
    logout_handler, me_handler, refresh_handler, require_role, update_user_handler,
};
use crate::domain::market_data::{CryptoProvider, EquityProvider};
use crate::domain::models::Role;
use crate::domain::repository::{HistoryRepo, NewUser, UserRepo};
use crate::infra::brapi::BrapiProvider;
use crate::infra::coinmarketcap::ReqwestCryptoProvider;
use crate::infra::finnhub::FinnhubProvider;
use crate::infra::sqlite::SqliteRepo;
use crate::targets_handlers::{
    get_barca_targets_handler, get_portfolio_targets_handler, save_barca_targets_handler,
    save_portfolio_targets_handler,
};
use crate::usecases::auth_service::hash_password;
use sqlx::SqlitePool;
use usecases::allocations_service::AllocationsService;
use usecases::history_service::HistoryService;

/// Everything an Axum handler needs. Composed once in `main` and shared
/// (cheaply, via `Arc`/`Clone`) across every request.
#[derive(Clone)]
struct AppState {
    crypto_provider: Arc<dyn CryptoProvider>,
    br_equity_provider: Arc<dyn EquityProvider>,
    us_equity_provider: Arc<dyn EquityProvider>,
    history_repo: Arc<SqliteRepo>,
    jwt_secret: Arc<Vec<u8>>,
}

#[tracing::instrument(skip(state))]
async fn api_allocations(
    _current: CurrentUser,
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
    // brapi/Finnhub keys are optional — only required once the user
    // actually tracks a br-equities/us-indices asset (see
    // AllocationsService::compute_and_record).
    let brapi_key = std::env::var("BRAPI_API_KEY").unwrap_or_default();
    let finnhub_key = std::env::var("FINNHUB_API_KEY").unwrap_or_default();

    let current_market =
        std::env::var("CURRENT_MARKET").unwrap_or_else(|_| "BullMarket".to_string());

    let alloc_svc = AllocationsService::new(
        state.crypto_provider.clone(),
        state.br_equity_provider.clone(),
        state.us_equity_provider.clone(),
        state.history_repo.clone(),
        state.history_repo.clone(),
    );
    let result = match alloc_svc
        .compute_and_record(&api_key, &brapi_key, &finnhub_key, &current_market)
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

    let history_svc = HistoryService::new(state.history_repo.clone(), state.history_repo.clone());
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
    _current: CurrentUser,
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> Json<serde_json::Value> {
    let level = q.level.unwrap_or_else(|| "totals".to_string());
    let svc = HistoryService::new(state.history_repo.clone(), state.history_repo.clone());
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
    current: CurrentUser,
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ImportPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
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
    ensure_admin_seeded(history_repo.clone()).await;

    let jwt_secret = Arc::new(
        std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set (e.g. `openssl rand -hex 32`)")
            .into_bytes(),
    );

    let crypto_provider = Arc::new(ReqwestCryptoProvider::new());
    let br_equity_provider = Arc::new(BrapiProvider::new());
    let us_equity_provider = Arc::new(FinnhubProvider::new());
    let app_state = AppState {
        crypto_provider,
        br_equity_provider,
        us_equity_provider,
        history_repo,
        jwt_secret,
    };

    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let cors = CorsLayer::new()
        .allow_origin(
            frontend_origin
                .parse::<axum::http::HeaderValue>()
                .expect("FRONTEND_ORIGIN must be a valid origin"),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .route("/api/allocations", get(api_allocations))
        .route("/api/history", get(api_history))
        .route(
            "/api/import_wallets",
            axum::routing::post(import_wallets_handler),
        )
        .route("/api/auth/login", axum::routing::post(login_handler))
        .route("/api/auth/logout", axum::routing::post(logout_handler))
        .route("/api/auth/refresh", axum::routing::post(refresh_handler))
        .route("/api/auth/me", get(me_handler))
        .route(
            "/api/admin/users",
            get(list_users_handler).post(create_user_handler),
        )
        .route(
            "/api/admin/users/{id}",
            axum::routing::patch(update_user_handler).delete(delete_user_handler),
        )
        .route(
            "/api/portfolio/targets",
            get(get_portfolio_targets_handler).put(save_portfolio_targets_handler),
        )
        .route(
            "/api/barca/targets",
            get(get_barca_targets_handler).put(save_barca_targets_handler),
        )
        .with_state(app_state)
        .layer(cors);

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
            let history_svc = HistoryService::new(history_repo.clone(), history_repo.clone());
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

/// There is no public self-registration endpoint anywhere in this app —
/// every account is created by an Admin. So on first boot, if no user
/// exists yet, seed one Admin from env vars rather than shipping a "create
/// your first account" flow nobody but the operator should ever see.
async fn ensure_admin_seeded(history_repo: Arc<SqliteRepo>) {
    let user_repo: Arc<dyn UserRepo> = history_repo.clone();
    match user_repo.count_users().await {
        Ok(0) => match (
            std::env::var("ADMIN_USERNAME"),
            std::env::var("ADMIN_PASSWORD"),
            std::env::var("ADMIN_EMAIL"),
        ) {
            (Ok(username), Ok(password), Ok(email)) => match hash_password(&password) {
                Ok(hash) => match user_repo
                    .create_user(NewUser {
                        username: &username,
                        password_hash: &hash,
                        role: Role::Admin.as_str(),
                        email: &email,
                        phone: None,
                    })
                    .await
                {
                    Ok(_) => info!(username = %username, "Bootstrapped initial admin account"),
                    Err(e) => error!(error = %e, "Failed to create bootstrap admin account"),
                },
                Err(e) => error!(error = %e, "Failed to hash bootstrap admin password"),
            },
            _ => warn!(
                "No admin account exists and ADMIN_USERNAME/ADMIN_PASSWORD/ADMIN_EMAIL are not all set — set them in .env to bootstrap one"
            ),
        },
        Ok(_) => {}
        Err(e) => warn!(error = %e, "Unable to check user count before admin bootstrap"),
    }
}

#[cfg(test)]
mod tests;
