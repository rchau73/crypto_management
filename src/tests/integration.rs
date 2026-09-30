//! End-to-end tests that drive the real Axum router in-process — no real
//! network or CoinMarketCap/brapi/Finnhub key required, only fakes for the
//! external market-data dependency. BARCA targets are seeded into the same
//! in-memory DB the router uses, since they're DB-authoritative.

use crate::auth_handlers::{login_handler, logout_handler, me_handler};
use crate::domain::market_data::MockEquityProvider;
use crate::domain::models::{MarketQuote, WalletAllocation};
use crate::domain::repository::{
    BarcaTargetInput, BarcaTargetRepo, HistoryRepo, NewUser, UserRepo,
};
use crate::infra::coinmarketcap::MockCryptoProvider;
use crate::usecases::auth_service::hash_password;
use crate::{
    AppState, api_allocations, api_history, correct_wallet_quantity_handler,
    get_portfolio_targets_handler, import_wallets_handler, import_wallets_upload_handler,
};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::routing::get;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::ServiceExt;

async fn body_json(res: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Every `Set-Cookie` header from a response, joined into one `Cookie` header
/// value suitable for the next request — this is what a real browser does.
fn cookie_header_from_response(res: &axum::response::Response) -> String {
    res.headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|set_cookie| set_cookie.split(';').next())
        .collect::<Vec<_>>()
        .join("; ")
}

async fn build_test_app() -> (Router, String) {
    let crypto = MarketQuote {
        symbol: "BTC".to_string(),
        price: 10.0,
        market_cap: 0.0,
        fdv: 0.0,
        volume_24h: 0.0,
        percent_change_24h: 0.0,
        percent_change_7d: 0.0,
    };

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to connect to in-memory db");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let repo = crate::infra::sqlite::repo::SqliteRepo::new(pool.clone());

    let wallet_rows = vec![
        WalletAllocation {
            id: None,
            symbol: "BTC".to_string(),
            group_name: Some("Base".to_string()),
            barca: Some("Base".to_string()),
            target_percent: Some(45.0),
            current_quantity: Some(1.0),
            last_price: Some(10.0),
            notes: Some("Ledger".to_string()),
            asset_class: "crypto".to_string(),
            created_at: None,
        },
        WalletAllocation {
            id: None,
            symbol: "BTC".to_string(),
            group_name: Some("Base".to_string()),
            barca: Some("Base".to_string()),
            target_percent: Some(0.0),
            current_quantity: Some(0.5),
            last_price: Some(10.0),
            notes: Some("Binance".to_string()),
            asset_class: "crypto".to_string(),
            created_at: None,
        },
    ];
    for wa in wallet_rows {
        repo.insert_wallet_allocation(&wa).await.unwrap();
    }

    repo.replace_barca_targets(
        "BullMarket",
        &[BarcaTargetInput {
            barca: "Base",
            target_percent: 100.0,
        }],
        None,
    )
    .await
    .unwrap();

    let password_hash = hash_password("correct-horse-battery-staple").unwrap();
    UserRepo::create_user(
        &repo,
        NewUser {
            username: "tester",
            password_hash: &password_hash,
            role: "manager",
            email: "tester@example.com",
            phone: None,
        },
    )
    .await
    .unwrap();
    UserRepo::create_user(
        &repo,
        NewUser {
            username: "viewer",
            password_hash: &password_hash,
            role: "user",
            email: "viewer@example.com",
            phone: None,
        },
    )
    .await
    .unwrap();

    let crypto_provider = Arc::new(MockCryptoProvider::new(vec![crypto]));
    let br_equity_provider = Arc::new(MockEquityProvider::new(vec![]));
    let us_equity_provider = Arc::new(MockEquityProvider::new(vec![]));
    let app_state = AppState {
        crypto_provider,
        br_equity_provider,
        us_equity_provider,
        history_repo: Arc::new(repo),
        jwt_secret: Arc::new(b"test-only-jwt-secret-do-not-use-in-prod".to_vec()),
    };

    let app = Router::new()
        .route("/api/allocations", get(api_allocations))
        .route("/api/history", get(api_history))
        .route(
            "/api/import_wallets",
            axum::routing::post(import_wallets_handler),
        )
        .route(
            "/api/import_wallets/upload",
            axum::routing::post(import_wallets_upload_handler),
        )
        .route("/api/portfolio/targets", get(get_portfolio_targets_handler))
        .route(
            "/api/portfolio/targets/quantity",
            axum::routing::put(correct_wallet_quantity_handler),
        )
        .route("/api/auth/login", axum::routing::post(login_handler))
        .route("/api/auth/logout", axum::routing::post(logout_handler))
        .route("/api/auth/me", get(me_handler))
        .with_state(app_state);

    (app, "correct-horse-battery-staple".to_string())
}

async fn login(app: &Router, password: &str) -> String {
    login_as(app, "tester", password).await
}

async fn login_as(app: &Router, username: &str, password: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"username":"{username}","password":"{password}"}}"#
        )))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success(),
        "login should succeed with the right password"
    );
    cookie_header_from_response(&res)
}

#[tokio::test]
async fn protected_routes_reject_requests_with_no_session_cookie() {
    let (app, _password) = build_test_app().await;

    let req = Request::builder()
        .uri("/api/allocations")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);

    let req = Request::builder()
        .uri("/api/history?level=totals")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_rejects_the_wrong_password_and_never_authorizes_the_caller() {
    let (app, _password) = build_test_app().await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"username":"tester","password":"totally-wrong"}"#,
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn allocations_and_history_round_trip_after_a_real_login_handshake() {
    unsafe {
        std::env::set_var("API_KEY", "test");
        std::env::remove_var("CURRENT_MARKET");
    }

    let (app, password) = build_test_app().await;
    let cookie = login(&app, &password).await;

    let req = Request::builder()
        .uri("/api/allocations")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let allocations_json = body_json(res).await;
    let per_asset = allocations_json["per_asset"].as_array().unwrap();
    assert_eq!(per_asset.len(), 1);
    assert_eq!(per_asset[0]["current_quantity"].as_f64().unwrap(), 1.5);

    let req_totals = Request::builder()
        .uri("/api/history?level=totals")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let totals_res = app.clone().oneshot(req_totals).await.unwrap();
    assert!(totals_res.status().is_success());
    let totals_json = body_json(totals_res).await;
    assert!(!totals_json["rows"].as_array().unwrap().is_empty());

    let req_assets = Request::builder()
        .uri("/api/history?level=assets")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let assets_res = app.clone().oneshot(req_assets).await.unwrap();
    assert!(assets_res.status().is_success());
    let assets_json = body_json(assets_res).await;
    let asset_rows = assets_json["rows"].as_array().unwrap();
    assert!(!asset_rows.is_empty());
    assert!(asset_rows[0].get("value_deviation").is_some());

    let req_groups = Request::builder()
        .uri("/api/history?level=groups")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let groups_res = app.clone().oneshot(req_groups).await.unwrap();
    assert!(groups_res.status().is_success());

    // `me` should reflect the account that just logged in.
    let req_me = Request::builder()
        .uri("/api/auth/me")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let me_res = app.clone().oneshot(req_me).await.unwrap();
    assert!(me_res.status().is_success());
    let me_json = body_json(me_res).await;
    assert_eq!(me_json["username"], "tester");
    assert_eq!(me_json["role"], "manager");
    assert_eq!(me_json["email"], "tester@example.com");

    // After logout, the same access-token cookie is still cryptographically
    // valid (it's short-lived and stateless) — but the refresh token backing
    // this session has been revoked, which is what logout actually protects.
    let req_logout = Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let logout_res = app.oneshot(req_logout).await.unwrap();
    assert!(logout_res.status().is_success());
}

/// A minimal `multipart/form-data` body with a single "file" field —
/// hand-built rather than pulled from a client library, since the point is
/// to drive the real Axum `Multipart` extractor end to end.
fn multipart_csv_body(csv: &str) -> (String, Body) {
    multipart_csv_body_bytes(csv.as_bytes())
}

/// Same shape as `multipart_csv_body`, but for raw bytes — lets a test send
/// something that isn't even valid UTF-8 text (e.g. binary file bytes).
fn multipart_csv_body_bytes(content: &[u8]) -> (String, Body) {
    multipart_body_named("wallet.csv", content)
}

/// Same shape again, but with an explicit filename — for tests exercising
/// the upload handler's filename-extension check.
fn multipart_body_named(filename: &str, content: &[u8]) -> (String, Body) {
    let boundary = "test-boundary-x7f3";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
             Content-Type: text/csv\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (
        format!("multipart/form-data; boundary={boundary}"),
        Body::from(body),
    )
}

#[tokio::test]
async fn csv_upload_is_rejected_for_the_viewer_role() {
    let (app, password) = build_test_app().await;
    let cookie = login_as(&app, "viewer", &password).await;

    let (content_type, body) = multipart_csv_body(
        "symbol,group,barca,target_percent,current_quantity,comments,asset_class\n\
         HGRU11,FII,Renda Variavel,5,10,seed,br-equities\n",
    );
    let req = Request::builder()
        .method("POST")
        .uri("/api/import_wallets/upload")
        .header(COOKIE, &cookie)
        .header("content-type", content_type)
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn csv_upload_imports_a_new_asset_for_a_manager() {
    let (app, password) = build_test_app().await;
    let cookie = login(&app, &password).await;

    let (content_type, body) = multipart_csv_body(
        "symbol,group,barca,target_percent,current_quantity,comments,asset_class\n\
         HGRU11,FII,Renda Variavel,5,10,seed,br-equities\n",
    );
    let req = Request::builder()
        .method("POST")
        .uri("/api/import_wallets/upload")
        .header(COOKIE, &cookie)
        .header("content-type", content_type)
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert!(
        res.status().is_success(),
        "upload should succeed for a manager"
    );
    let json = body_json(res).await;
    assert_eq!(json["imported"], 1);
}

#[tokio::test]
async fn csv_upload_of_a_binary_file_is_rejected_by_content_sniffing_before_parsing() {
    let (app, password) = build_test_app().await;
    let cookie = login(&app, &password).await;

    // JPEG magic bytes + garbage — not valid UTF-8 at all. The upload
    // handler's binary-content check must catch this itself, before the
    // file ever reaches the CSV parser.
    let (content_type, body) =
        multipart_csv_body_bytes(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46]);
    let req = Request::builder()
        .method("POST")
        .uri("/api/import_wallets/upload")
        .header(COOKIE, &cookie)
        .header("content-type", content_type)
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        axum::http::StatusCode::BAD_REQUEST,
        "a bad upload is the caller's mistake, not a 500"
    );
    let json = body_json(res).await;
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("binary content detected")
    );
}

#[tokio::test]
async fn csv_upload_rejects_a_non_csv_filename_before_looking_at_content() {
    let (app, password) = build_test_app().await;
    let cookie = login(&app, &password).await;

    let (content_type, body) =
        multipart_body_named("wallet.docx", b"symbol,group,barca\nBTC,Core,Base\n");
    let req = Request::builder()
        .method("POST")
        .uri("/api/import_wallets/upload")
        .header(COOKIE, &cookie)
        .header("content-type", content_type)
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
    let json = body_json(res).await;
    assert!(json["error"].as_str().unwrap().contains("wallet.docx"));
}

#[tokio::test]
async fn csv_upload_with_a_missing_required_column_is_rejected_as_a_client_error() {
    let (app, password) = build_test_app().await;
    let cookie = login(&app, &password).await;

    // Well-formed CSV text, but no "symbol" column — WalletCsvRow requires it.
    let (content_type, body) = multipart_csv_body("group,barca\nCore,Base\n");
    let req = Request::builder()
        .method("POST")
        .uri("/api/import_wallets/upload")
        .header(COOKIE, &cookie)
        .header("content-type", content_type)
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

fn correct_quantity_body(notes: &str, new_quantity: f64) -> Body {
    Body::from(format!(
        r#"{{"symbol":"BTC","group_name":"Base","barca":"Base","asset_class":"crypto","notes":"{notes}","current_quantity":{new_quantity}}}"#
    ))
}

#[tokio::test]
async fn correct_wallet_quantity_is_rejected_for_the_viewer_role() {
    let (app, password) = build_test_app().await;
    let cookie = login_as(&app, "viewer", &password).await;

    let req = Request::builder()
        .method("PUT")
        .uri("/api/portfolio/targets/quantity")
        .header(COOKIE, &cookie)
        .header("content-type", "application/json")
        .body(correct_quantity_body("Ledger", 3.0))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn correct_wallet_quantity_updates_only_the_named_source_for_a_manager() {
    let (app, password) = build_test_app().await;
    let cookie = login(&app, &password).await;

    // build_test_app seeds BTC from two sources: "Ledger" (1.0) and
    // "Binance" (0.5). Correcting only "Ledger" to 3.0 must leave
    // "Binance" untouched, landing on 3.0 + 0.5, never 1.0 + 0.5 + 3.0.
    let req = Request::builder()
        .method("PUT")
        .uri("/api/portfolio/targets/quantity")
        .header(COOKIE, &cookie)
        .header("content-type", "application/json")
        .body(correct_quantity_body("Ledger", 3.0))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success(),
        "correction should succeed for a manager"
    );

    let get_req = Request::builder()
        .uri("/api/portfolio/targets")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let get_res = app.oneshot(get_req).await.unwrap();
    let rows = body_json(get_res).await;
    let btc = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "BTC")
        .unwrap();
    assert_eq!(btc["current_quantity"], 3.5);
}
