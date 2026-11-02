//! End-to-end tests through the real router (`api::build_router`) with an
//! in-memory database and fake price providers — no network needed.

use crate::api::{AppState, build_router};
use crate::config::AppConfig;
use crate::domain::market_data::{FakeFx, FakeProvider};
use crate::domain::models::{
    LedgerEntry, NewBarcaTarget, NewPortfolioTarget, PortfolioAccess, PortfolioId, PositionKey,
};
use crate::domain::repository::{
    BarcaTargetRepo, NewUser, PortfolioDirectoryRepo, PortfolioRepo, UserRepo,
};
use crate::infra::sqlite::test_repo;
use crate::usecases::allocations_service::MarketProviders;
use crate::usecases::auth_service::hash_password;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const PASSWORD: &str = "correct-horse-battery-staple";

/// The portfolio the migration creates; `test_app` seeds it.
const PORTFOLIO: PortfolioId = PortfolioId::FIRST;

fn ledger(symbol: &str, qty: f64, notes: &str) -> LedgerEntry {
    LedgerEntry {
        symbol: symbol.to_string(),
        group_name: Some("Base".to_string()),
        barca: Some("Base".to_string()),
        asset_class: "crypto".to_string(),
        current_quantity: qty,
        last_price: Some(10.0),
        notes: Some(notes.to_string()),
    }
}

fn target(symbol: &str, target_percent: f64) -> NewPortfolioTarget {
    NewPortfolioTarget {
        key: PositionKey {
            symbol: symbol.to_string(),
            group_name: "Base".to_string(),
            barca: "Base".to_string(),
            asset_class: "crypto".to_string(),
        },
        target_percent,
    }
}

/// A router over a database holding:
/// - portfolio 1 with BTC from two sources (Ledger 1.0 + Binance 0.5) and
///   ETH from one (Kraken 2.0), targets 60/40, BARCA "Base" = 100% in
///   BullMarket;
/// - users "admin" (admin), "manager" (manager) and "viewer" (user), members
///   of portfolio 1 (manage, manage, view), and "outsider" (manager) who
///   isn't a member of anything.
async fn test_app(crypto: FakeProvider) -> Router {
    let repo = test_repo().await;
    repo.import_positions(
        PORTFOLIO,
        &[target("BTC", 60.0), target("ETH", 40.0)],
        &[
            ledger("BTC", 1.0, "Ledger"),
            ledger("BTC", 0.5, "Binance"),
            ledger("ETH", 2.0, "Kraken"),
        ],
    )
    .await
    .unwrap();
    repo.replace_barca_targets(
        PORTFOLIO,
        "BullMarket",
        &[NewBarcaTarget {
            barca: "Base".to_string(),
            target_percent: 100.0,
        }],
        None,
    )
    .await
    .unwrap();

    let hash = hash_password(PASSWORD).unwrap();
    for (username, role, access) in [
        ("admin", "admin", Some(PortfolioAccess::Manage)),
        ("manager", "manager", Some(PortfolioAccess::Manage)),
        ("viewer", "user", Some(PortfolioAccess::View)),
        ("outsider", "manager", None),
    ] {
        let user = repo
            .create_user(NewUser {
                username,
                password_hash: &hash,
                role,
                email: &format!("{username}@example.com"),
                phone: None,
            })
            .await
            .unwrap();
        if let Some(access) = access {
            repo.set_member(PORTFOLIO, user.id.unwrap(), access)
                .await
                .unwrap();
        }
    }

    let providers = MarketProviders {
        crypto: Arc::new(crypto),
        br_equities: Arc::new(FakeProvider::empty()),
        us_indices: Arc::new(FakeProvider::empty()),
        usd_brl: Arc::new(FakeFx(Some(5.0))),
    };
    build_router(AppState::new(repo, providers, &AppConfig::for_tests()))
}

async fn default_app() -> Router {
    test_app(FakeProvider::with_prices(&[("BTC", 10.0), ("ETH", 5.0)])).await
}

/// Sends a request and returns (status, JSON body).
async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn get(uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

fn with_json(method: &str, uri: &str, cookie: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(COOKIE, cookie)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Logs in and returns the session cookies, as a browser would send them.
async fn login(app: &Router, username: &str) -> String {
    let request = with_json(
        "POST",
        "/api/auth/login",
        "",
        json!({ "username": username, "password": PASSWORD }),
    );
    let response = app.clone().oneshot(request).await.unwrap();
    assert!(response.status().is_success(), "login as {username}");
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok()?.split(';').next())
        .collect::<Vec<_>>()
        .join("; ")
}

fn csv_upload(cookie: &str, filename: &str, content: &[u8]) -> Request<Body> {
    csv_upload_to(PORTFOLIO, cookie, filename, content)
}

fn csv_upload_to(
    portfolio: PortfolioId,
    cookie: &str,
    filename: &str,
    content: &[u8],
) -> Request<Body> {
    let boundary = "test-boundary-x7f3";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
         Content-Type: text/csv\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri(format!("/api/portfolios/{portfolio}/import_wallets/upload"))
        .header(COOKIE, cookie)
        .header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap()
}

async fn btc_quantity(app: &Router, cookie: &str) -> f64 {
    let (_, rows) = send(app, get("/api/portfolios/1/targets", cookie)).await;
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "BTC")
        .unwrap()["current_quantity"]
        .as_f64()
        .unwrap()
}

// --- authentication --------------------------------------------------------

#[tokio::test]
async fn protected_routes_require_a_session() {
    let app = default_app().await;
    for uri in [
        "/api/portfolios/1/allocations",
        "/api/portfolios/1/history",
        "/api/portfolios/1/targets",
        "/api/admin/users",
    ] {
        let (status, _) = send(&app, get(uri, "")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let (status, _) = send(
        &app,
        get("/api/portfolios/1/allocations", "cm_access_token=forged"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_sets_cookies_named_with_the_configured_prefix() {
    let app = default_app().await;
    let cookie = login(&app, "viewer").await;
    // The cookie jar doesn't keep insertion order, so compare sorted.
    let mut names: Vec<_> = cookie
        .split("; ")
        .filter_map(|pair| pair.split('=').next())
        .collect();
    names.sort();
    assert_eq!(names, ["cm_access_token", "cm_refresh_token"]);
}

/// Another local app (same host, different port) using the plain cookie
/// names must neither authenticate here nor break this app's session.
#[tokio::test]
async fn another_apps_cookies_are_ignored() {
    let app = default_app().await;

    let foreign_only = "access_token=other-app; refresh_token=other-app";
    let (status, _) = send(&app, get("/api/auth/me", foreign_only)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let both = format!("{foreign_only}; {}", login(&app, "viewer").await);
    let (status, me) = send(&app, get("/api/auth/me", &both)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["username"], "viewer");

    let refresh = Request::builder()
        .method("POST")
        .uri("/api/auth/refresh")
        .header(COOKIE, &both)
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&app, refresh).await.0, StatusCode::OK);
}

#[tokio::test]
async fn logout_clears_only_this_apps_cookies() {
    let app = default_app().await;
    let cookie = login(&app, "viewer").await;
    let logout = Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header(COOKIE, format!("access_token=other-app; {cookie}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(logout).await.unwrap();
    let mut cleared: Vec<_> = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok()?.split('=').next().map(str::to_string))
        .collect();
    cleared.sort();
    assert_eq!(cleared, ["cm_access_token", "cm_refresh_token"]);
}

#[tokio::test]
async fn a_wrong_password_is_rejected() {
    let app = default_app().await;
    let request = with_json(
        "POST",
        "/api/auth/login",
        "",
        json!({ "username": "manager", "password": "wrong" }),
    );
    let (status, body) = send(&app, request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid username or password");
}

#[tokio::test]
async fn me_refresh_and_logout_work_with_the_session_cookies() {
    let app = default_app().await;
    let cookie = login(&app, "manager").await;

    let (status, me) = send(&app, get("/api/auth/me", &cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        me,
        json!({ "username": "manager", "role": "manager", "email": "manager@example.com" })
    );

    let refresh = Request::builder()
        .method("POST")
        .uri("/api/auth/refresh")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&app, refresh).await.0, StatusCode::OK);

    let logout = Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&app, logout).await.0, StatusCode::OK);

    let refresh_after_logout = Request::builder()
        .method("POST")
        .uri("/api/auth/refresh")
        .header(COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        send(&app, refresh_after_logout).await.0,
        StatusCode::UNAUTHORIZED
    );
}

// --- allocations and history --------------------------------------------------

#[tokio::test]
async fn update_prices_then_every_history_level_has_rows() {
    let app = default_app().await;
    let cookie = login(&app, "viewer").await;

    let (status, report) = send(&app, get("/api/portfolios/1/allocations", &cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["total_value"], 25.0); // 1.5 BTC * 10 + 2 ETH * 5
    assert_eq!(report["per_asset"][0]["symbol"], "BTC");
    assert_eq!(report["per_asset"][0]["current_quantity"], 1.5);
    assert_eq!(report["per_barca"][0]["target_percent"], 100.0);

    for level in ["totals", "assets", "groups", "barca"] {
        let (status, history) = send(
            &app,
            get(&format!("/api/portfolios/1/history?level={level}"), &cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{level}");
        assert_eq!(history["level"], level);
        assert!(!history["rows"].as_array().unwrap().is_empty(), "{level}");
    }
}

#[tokio::test]
async fn an_unknown_history_level_is_a_400_not_a_silent_fallback() {
    let app = default_app().await;
    let cookie = login(&app, "viewer").await;
    let (status, body) = send(&app, get("/api/portfolios/1/history?level=weekly", &cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("weekly"));
}

#[tokio::test]
async fn a_crypto_price_outage_is_a_502_with_the_reason() {
    let app = test_app(FakeProvider::failing()).await;
    let cookie = login(&app, "viewer").await;
    let (status, body) = send(&app, get("/api/portfolios/1/allocations", &cookie)).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(body["error"].as_str().unwrap().contains("crypto prices"));
}

// --- portfolio targets ------------------------------------------------------

#[tokio::test]
async fn portfolio_targets_list_includes_the_source_count() {
    let app = default_app().await;
    let cookie = login(&app, "viewer").await;
    let (_, rows) = send(&app, get("/api/portfolios/1/targets", &cookie)).await;
    let btc = &rows[0];
    assert_eq!(btc["symbol"], "BTC");
    assert_eq!(btc["source_count"], 2);
    assert_eq!(btc["target_percent"], 60.0);
}

#[tokio::test]
async fn saving_portfolio_targets_needs_manager_and_a_valid_set() {
    let app = default_app().await;
    let rows = |btc: f64, eth: f64| {
        json!({ "rows": [
            { "symbol": "BTC", "group_name": "Base", "barca": "Base", "asset_class": "crypto", "target_percent": btc },
            { "symbol": "ETH", "group_name": "Base", "barca": "Base", "asset_class": "crypto", "target_percent": eth },
        ]})
    };

    let viewer = login(&app, "viewer").await;
    let (status, _) = send(
        &app,
        with_json(
            "PUT",
            "/api/portfolios/1/targets",
            &viewer,
            rows(50.0, 50.0),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let manager = login(&app, "manager").await;
    let (status, body) = send(
        &app,
        with_json(
            "PUT",
            "/api/portfolios/1/targets",
            &manager,
            rows(50.0, 30.0),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("100%"));

    let (status, _) = send(
        &app,
        with_json(
            "PUT",
            "/api/portfolios/1/targets",
            &manager,
            rows(50.0, 50.0),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        btc_quantity(&app, &manager).await,
        1.5,
        "quantities untouched"
    );
}

#[tokio::test]
async fn correcting_quantity_works_for_one_source_and_is_refused_for_several() {
    let app = default_app().await;
    let body = |symbol: &str, qty: f64| json!({ "symbol": symbol, "group_name": "Base", "barca": "Base", "asset_class": "crypto", "current_quantity": qty });

    let viewer = login(&app, "viewer").await;
    let (status, _) = send(
        &app,
        with_json(
            "PUT",
            "/api/portfolios/1/targets/quantity",
            &viewer,
            body("ETH", 3.0),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let manager = login(&app, "manager").await;
    let (status, _) = send(
        &app,
        with_json(
            "PUT",
            "/api/portfolios/1/targets/quantity",
            &manager,
            body("ETH", 3.0),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // BTC has two sources: writing one number would be ambiguous.
    let (status, _) = send(
        &app,
        with_json(
            "PUT",
            "/api/portfolios/1/targets/quantity",
            &manager,
            body("BTC", 9.0),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(btc_quantity(&app, &manager).await, 1.5);

    let (status, _) = send(
        &app,
        with_json(
            "PUT",
            "/api/portfolios/1/targets/quantity",
            &manager,
            body("GHOST", 1.0),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn barca_targets_round_trip_for_a_manager() {
    let app = default_app().await;
    let manager = login(&app, "manager").await;
    let payload = json!({ "market": "BearMarket", "targets": [
        { "barca": "Base", "target_percent": 70.0 },
        { "barca": "Caixa", "target_percent": 30.0 },
    ]});

    let (status, _) = send(
        &app,
        with_json("PUT", "/api/portfolios/1/barca/targets", &manager, payload),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, targets) = send(
        &app,
        get(
            "/api/portfolios/1/barca/targets?market=BearMarket",
            &manager,
        ),
    )
    .await;
    assert_eq!(targets.as_array().unwrap().len(), 2);
}

// --- CSV upload -----------------------------------------------------------------

const NEW_ASSET_CSV: &[u8] =
    b"symbol,group,barca,target_percent,current_quantity,notes,asset_class\n\
    HGRU11,FII,Renda Variavel,5,10,seed,br-equities\n";

#[tokio::test]
async fn csv_upload_needs_manager_and_imports_new_positions() {
    let app = default_app().await;
    let viewer = login(&app, "viewer").await;
    let (status, _) = send(&app, csv_upload(&viewer, "wallet.csv", NEW_ASSET_CSV)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let manager = login(&app, "manager").await;
    let (status, body) = send(&app, csv_upload(&manager, "wallet.csv", NEW_ASSET_CSV)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["positions_added"], 1);
    assert_eq!(body["sources_added"], 1);
    // The seeded BTC (2 sources) and ETH aren't in this CSV: reported, not zeroed.
    assert_eq!(body["missing_from_csv"].as_array().unwrap().len(), 3);
    assert_eq!(body["missing_from_csv"][0]["symbol"], "BTC");
}

#[tokio::test]
async fn bad_csv_uploads_are_400s_with_a_useful_message() {
    let app = default_app().await;
    let manager = login(&app, "manager").await;
    let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

    let cases: [(&str, &[u8], &str); 4] = [
        ("wallet.docx", b"symbol\nBTC\n", "wallet.docx"),
        ("wallet.csv", &jpeg, "binary content detected"),
        (
            "wallet.csv",
            b"group,barca\nCore,Base\n",
            "\"symbol\" column",
        ),
        (
            "wallet.csv",
            b"symbol,current_quantity\nBTC,1\nETH,-3\n",
            "line 3",
        ),
    ];
    for (filename, content, expected) in cases {
        let (status, body) = send(&app, csv_upload(&manager, filename, content)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{expected}");
        let message = body["error"].as_str().unwrap();
        assert!(message.contains(expected), "{message}");
    }
}

#[tokio::test]
async fn wallet_export_needs_manager_and_downloads_the_import_format() {
    let app = default_app().await;
    let (status, _) = send(&app, get("/api/portfolios/1/export_wallets", "")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let viewer = login(&app, "viewer").await;
    let (status, _) = send(&app, get("/api/portfolios/1/export_wallets", &viewer)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let manager = login(&app, "manager").await;
    send(&app, get("/api/portfolios/1/allocations", &manager)).await; // record prices
    let response = app
        .clone()
        .oneshot(get("/api/portfolios/1/export_wallets", &manager))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(headers[CONTENT_TYPE], "text/csv; charset=utf-8");
    let disposition = headers["content-disposition"].to_str().unwrap();
    assert!(disposition.starts_with("attachment; filename=\"wallet_allocations-"));
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let csv = String::from_utf8(bytes.to_vec()).unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some(
            "symbol,group,barca,target_percent,current_quantity,comments,asset_class,price_usd,value_usd"
        )
    );
    assert!(
        csv.lines()
            .any(|l| l.starts_with("BTC,") && l.contains(",10,")),
        "{csv}"
    );
}

// --- user administration ------------------------------------------------------

#[tokio::test]
async fn only_admins_manage_users() {
    let app = default_app().await;
    let manager = login(&app, "manager").await;
    let (status, _) = send(&app, get("/api/admin/users", &manager)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let admin = login(&app, "admin").await;
    let (status, users) = send(&app, get("/api/admin/users", &admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(users.as_array().unwrap().len(), 4);
    assert!(
        users[0].get("password_hash").is_none(),
        "hash never leaves the server"
    );
}

#[tokio::test]
async fn admin_user_lifecycle_and_error_codes() {
    let app = default_app().await;
    let admin = login(&app, "admin").await;
    let new_user = json!({ "username": "erin", "password": "pw123456", "role": "user", "email": "erin@example.com" });

    let (status, created) = send(
        &app,
        with_json("POST", "/api/admin/users", &admin, new_user.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["id"].as_i64().unwrap();

    let (status, _) = send(
        &app,
        with_json("POST", "/api/admin/users", &admin, new_user),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "duplicate username");

    let promote = json!({ "role": "manager" });
    let (status, _) = send(
        &app,
        with_json(
            "PATCH",
            &format!("/api/admin/users/{id}"),
            &admin,
            promote.clone(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(
        &app,
        with_json("PATCH", "/api/admin/users/9999", &admin, promote),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let delete = |id: i64| {
        Request::builder()
            .method("DELETE")
            .uri(format!("/api/admin/users/{id}"))
            .header(COOKIE, &admin)
            .body(Body::empty())
            .unwrap()
    };
    assert_eq!(send(&app, delete(id)).await.0, StatusCode::OK);
    assert_eq!(send(&app, delete(id)).await.0, StatusCode::NOT_FOUND);

    let (_, me) = send(&app, get("/api/admin/users", &admin)).await;
    let admin_id = me
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["username"] == "admin")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    assert_eq!(
        send(&app, delete(admin_id)).await.0,
        StatusCode::BAD_REQUEST,
        "no self-delete"
    );
}

// --- separate portfolios -------------------------------------------------------

/// Creates a portfolio as `cookie` and returns its id.
async fn create_portfolio(app: &Router, cookie: &str, body: Value) -> PortfolioId {
    let (status, created) = send(app, with_json("POST", "/api/portfolios", cookie, body)).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    PortfolioId(created["id"].as_i64().unwrap())
}

fn symbols(rows: &Value) -> Vec<&str> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r["symbol"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn the_portfolio_list_only_has_the_callers_own() {
    let app = default_app().await;
    let manager = login(&app, "manager").await;
    let (status, list) = send(&app, get("/api/portfolios", &manager)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        list,
        json!([{ "id": 1, "name": "My portfolio", "access": "manage" }])
    );

    let outsider = login(&app, "outsider").await;
    let (_, list) = send(&app, get("/api/portfolios", &outsider)).await;
    assert_eq!(list, json!([]));
}

#[tokio::test]
async fn a_non_member_gets_404_on_every_route_of_the_portfolio() {
    // 404, not 403: the API must not even reveal that the portfolio exists.
    let app = default_app().await;
    let outsider = login(&app, "outsider").await;
    let gets = [
        "/api/portfolios/1/allocations",
        "/api/portfolios/1/history",
        "/api/portfolios/1/targets",
        "/api/portfolios/1/barca/targets?market=BullMarket",
        "/api/portfolios/1/export_wallets",
    ];
    for uri in gets {
        let (status, _) = send(&app, get(uri, &outsider)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
    let writes = [
        with_json(
            "PUT",
            "/api/portfolios/1/targets",
            &outsider,
            json!({ "rows": [] }),
        ),
        with_json(
            "PUT",
            "/api/portfolios/1/targets/quantity",
            &outsider,
            json!({ "symbol": "ETH", "group_name": "Base", "barca": "Base", "asset_class": "crypto", "current_quantity": 9.0 }),
        ),
        with_json(
            "PUT",
            "/api/portfolios/1/barca/targets",
            &outsider,
            json!({ "market": "BullMarket", "targets": [{ "barca": "Base", "target_percent": 100.0 }] }),
        ),
        with_json(
            "PATCH",
            "/api/portfolios/1",
            &outsider,
            json!({ "name": "Mine" }),
        ),
        csv_upload(&outsider, "wallet.csv", NEW_ASSET_CSV),
    ];
    for request in writes {
        let uri = request.uri().to_string();
        let (status, _) = send(&app, request).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
    // And nothing changed.
    let manager = login(&app, "manager").await;
    assert_eq!(btc_quantity(&app, &manager).await, 1.5);
}

#[tokio::test]
async fn a_portfolio_that_does_not_exist_is_a_404() {
    let app = default_app().await;
    let manager = login(&app, "manager").await;
    for uri in [
        "/api/portfolios/999/targets",
        "/api/portfolios/not-a-number/targets",
    ] {
        let (status, _) = send(&app, get(uri, &manager)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn creating_and_renaming_need_a_manager() {
    let app = default_app().await;
    let viewer = login(&app, "viewer").await;
    let (status, _) = send(
        &app,
        with_json(
            "POST",
            "/api/portfolios",
            &viewer,
            json!({ "name": "Spouse" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = send(
        &app,
        with_json(
            "PATCH",
            "/api/portfolios/1",
            &viewer,
            json!({ "name": "Mine" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "view access can't rename");

    let manager = login(&app, "manager").await;
    let (status, _) = send(
        &app,
        with_json(
            "POST",
            "/api/portfolios",
            &manager,
            json!({ "name": "My portfolio" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "names are unique");
    let (status, _) = send(
        &app,
        with_json(
            "PATCH",
            "/api/portfolios/1",
            &manager,
            json!({ "name": "Roberto" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, list) = send(&app, get("/api/portfolios", &viewer)).await;
    assert_eq!(list[0]["name"], "Roberto", "renamed for every member");
}

#[tokio::test]
async fn two_portfolios_never_share_positions_targets_history_or_exports() {
    let app = default_app().await;
    let manager = login(&app, "manager").await;
    let spouse = create_portfolio(
        &app,
        &manager,
        json!({ "name": "Spouse", "copy_barca_from": 1 }),
    )
    .await;

    // A new portfolio starts empty, with a copy of the BARCA targets.
    let (_, rows) = send(
        &app,
        get(&format!("/api/portfolios/{spouse}/targets"), &manager),
    )
    .await;
    assert_eq!(rows, json!([]));
    let (_, barca) = send(
        &app,
        get(
            &format!("/api/portfolios/{spouse}/barca/targets?market=BullMarket"),
            &manager,
        ),
    )
    .await;
    assert_eq!(barca[0]["barca"], "Base");

    // Import into the spouse's portfolio only.
    let csv = b"symbol,group,barca,target_percent,current_quantity,notes,asset_class\n\
                ETH,Base,Base,100,7,Binance,crypto\n";
    let (status, summary) = send(&app, csv_upload_to(spouse, &manager, "wallet.csv", csv)).await;
    assert_eq!(status, StatusCode::OK, "{summary}");
    assert_eq!(
        summary["positions_added"], 1,
        "ETH is new *in this portfolio*"
    );
    assert_eq!(
        summary["missing_from_csv"],
        json!([]),
        "portfolio 1's sources aren't involved"
    );

    let (_, mine) = send(&app, get("/api/portfolios/1/targets", &manager)).await;
    let (_, theirs) = send(
        &app,
        get(&format!("/api/portfolios/{spouse}/targets"), &manager),
    )
    .await;
    assert_eq!(symbols(&mine), ["BTC", "ETH"]);
    assert_eq!(theirs[0]["current_quantity"], 7.0);
    let my_eth = mine
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "ETH")
        .unwrap();
    assert_eq!(
        my_eth["current_quantity"], 2.0,
        "my ETH didn't pick up their 7"
    );

    // Prices and history are per portfolio too.
    let (status, report) = send(
        &app,
        get(&format!("/api/portfolios/{spouse}/allocations"), &manager),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["total_value"], 35.0, "7 ETH x 5, nothing of mine");
    let (_, my_history) = send(
        &app,
        get("/api/portfolios/1/history?level=totals", &manager),
    )
    .await;
    let (_, their_history) = send(
        &app,
        get(
            &format!("/api/portfolios/{spouse}/history?level=totals"),
            &manager,
        ),
    )
    .await;
    assert_eq!(
        my_history["rows"],
        json!([]),
        "their refresh isn't in my history"
    );
    assert_eq!(their_history["rows"].as_array().unwrap().len(), 1);

    // Saving their targets leaves mine alone.
    let (status, _) = send(
        &app,
        with_json(
            "PUT",
            &format!("/api/portfolios/{spouse}/targets"),
            &manager,
            json!({ "rows": [{ "symbol": "ETH", "group_name": "Base", "barca": "Base", "asset_class": "crypto", "target_percent": 100.0 }] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, mine_after) = send(&app, get("/api/portfolios/1/targets", &manager)).await;
    assert_eq!(mine_after, mine);

    // The export only has my rows.
    send(&app, get("/api/portfolios/1/allocations", &manager)).await;
    let response = app
        .clone()
        .oneshot(get("/api/portfolios/1/export_wallets", &manager))
        .await
        .unwrap();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let csv = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!csv.contains(",7,"), "{csv}");
    assert_eq!(csv.lines().count(), 4, "header + BTC x2 + ETH: {csv}");
}

#[tokio::test]
async fn a_new_portfolio_is_invisible_to_the_other_members_of_mine() {
    // The viewer can read portfolio 1, but a portfolio the manager creates
    // later is the manager's alone.
    let app = default_app().await;
    let manager = login(&app, "manager").await;
    let spouse = create_portfolio(&app, &manager, json!({ "name": "Spouse" })).await;

    let viewer = login(&app, "viewer").await;
    let (_, list) = send(&app, get("/api/portfolios", &viewer)).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let (status, _) = send(
        &app,
        get(&format!("/api/portfolios/{spouse}/targets"), &viewer),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let admin = login(&app, "admin").await;
    let (status, _) = send(
        &app,
        get(&format!("/api/portfolios/{spouse}/targets"), &admin),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "admins only see portfolios they're members of"
    );
}
