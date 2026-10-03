//! Server entry point: read config, open the database, wire services,
//! seed first-run data and serve HTTP until Ctrl+C.

use anyhow::Context;
use axum::http::{HeaderValue, Method, header};
use crypto_management::api::{AppState, build_router};
use crypto_management::config::AppConfig;
use crypto_management::infra::bcb_ptax::BcbPtaxProvider;
use crypto_management::infra::brapi::BrapiProvider;
use crypto_management::infra::coinmarketcap::CoinMarketCapProvider;
use crypto_management::infra::finnhub::FinnhubProvider;
use crypto_management::infra::sqlite::{self, SqliteRepo};
use crypto_management::usecases::allocations_service::MarketProviders;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    // Log level comes from RUST_LOG (e.g. `RUST_LOG=debug`), default info.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::from_env()?;

    let pool = sqlite::connect(&config.database_url)
        .await
        .with_context(|| format!("failed to open database {}", config.database_url))?;
    info!("Database ready, migrations applied");

    let providers = MarketProviders {
        crypto: Arc::new(CoinMarketCapProvider::new(
            config.coinmarketcap_api_key.clone(),
        )),
        br_equities: Arc::new(BrapiProvider::new(config.brapi_api_key.clone())),
        us_indices: Arc::new(FinnhubProvider::new(config.finnhub_api_key.clone())),
        usd_brl: Arc::new(BcbPtaxProvider::new()),
    };
    let state = AppState::new(Arc::new(SqliteRepo::new(pool)), providers, &config);

    seed_first_run_data(&state, &config).await;

    let app = build_router(state).layer(cors_layer(&config)?);
    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    info!(addr = %config.bind_addr, "Listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// On an empty database: seed the portfolio from the wallet CSV and create
/// the admin account (there is no public sign-up).
async fn seed_first_run_data(state: &AppState, config: &AppConfig) {
    state
        .wallet_import
        .seed_if_empty(&config.wallet_seed_path)
        .await;

    let Some(admin) = &config.admin_seed else {
        return;
    };
    match state
        .users
        .ensure_admin_exists(&admin.username, &admin.password, &admin.email)
        .await
    {
        Ok(true) => info!(username = %admin.username, "Created the initial admin account"),
        Ok(false) => {}
        Err(e) => warn!(error = %e, "Failed to create the initial admin account"),
    }
}

/// The browser only sends cookies cross-origin to an exact, allowed origin.
fn cors_layer(config: &AppConfig) -> anyhow::Result<CorsLayer> {
    let origin: HeaderValue = config
        .frontend_origin
        .parse()
        .context("FRONTEND_ORIGIN must be a valid origin")?;
    Ok(CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true))
}

async fn shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        warn!(error = %e, "Could not listen for Ctrl+C");
        std::future::pending::<()>().await;
    }
    info!("Shutting down");
}
