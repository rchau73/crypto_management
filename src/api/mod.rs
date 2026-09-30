//! The HTTP layer: routes, shared state and error handling. Handlers stay
//! thin — they check permissions, call one service and shape the response.

pub mod auth;
pub mod error;
pub mod portfolio;
pub mod users;

use crate::config::AppConfig;
use crate::infra::sqlite::SqliteRepo;
use crate::usecases::allocations_service::{AllocationsService, MarketProviders};
use crate::usecases::auth_service::AuthService;
use crate::usecases::history_service::HistoryService;
use crate::usecases::targets_service::TargetsService;
use crate::usecases::user_service::UserService;
use crate::usecases::wallet_import::WalletImportService;
use axum::Router;
use axum::routing::{get, patch, post, put};
use std::sync::Arc;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Every service a handler may need, built once at startup and shared
/// (cheaply, via `Arc`) by all requests.
#[derive(Clone)]
pub struct AppState {
    pub allocations: Arc<AllocationsService>,
    pub history: Arc<HistoryService>,
    pub targets: Arc<TargetsService>,
    pub wallet_import: Arc<WalletImportService>,
    pub auth: Arc<AuthService>,
    pub users: Arc<UserService>,
    pub cookie_secure: bool,
}

impl AppState {
    /// Wires every service to its dependencies. This is the only place that
    /// knows the concrete types (SQLite, the real price providers); every
    /// service only sees the traits it needs.
    pub fn new(repo: Arc<SqliteRepo>, providers: MarketProviders, config: &AppConfig) -> Self {
        Self {
            allocations: Arc::new(AllocationsService::new(
                providers,
                repo.clone(),
                repo.clone(),
                repo.clone(),
                config.current_market.clone(),
            )),
            history: Arc::new(HistoryService::new(repo.clone())),
            targets: Arc::new(TargetsService::new(repo.clone(), repo.clone())),
            wallet_import: Arc::new(WalletImportService::new(repo.clone())),
            auth: Arc::new(AuthService::new(
                repo.clone(),
                repo.clone(),
                Arc::new(config.jwt_secret.clone()),
            )),
            users: Arc::new(UserService::new(repo)),
            cookie_secure: config.cookie_secure,
        }
    }
}

/// All routes. Used by `main` and by the integration tests, so tests
/// always exercise the real routing table.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/auth/me", get(auth::me))
        .route("/api/admin/users", get(users::list).post(users::create))
        .route(
            "/api/admin/users/{id}",
            patch(users::update).delete(users::delete),
        )
        .route("/api/allocations", get(portfolio::allocations))
        .route("/api/history", get(portfolio::history))
        .route(
            "/api/portfolio/targets",
            get(portfolio::get_portfolio_targets).put(portfolio::save_portfolio_targets),
        )
        .route(
            "/api/portfolio/targets/quantity",
            put(portfolio::correct_quantity),
        )
        .route(
            "/api/barca/targets",
            get(portfolio::get_barca_targets).put(portfolio::save_barca_targets),
        )
        .route(
            "/api/import_wallets/upload",
            post(portfolio::upload_wallet_csv),
        )
        // One span per request (method, path, status, latency), so every
        // log line inside a handler can be traced back to its request.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}
