//! End-to-end tests that drive the real Axum router in-process — no real
//! network or CoinMarketCap key required, only fakes for the two external
//! dependencies (market data, wallet CSV targets).

use crate::csv_store::AllocationStore;
use crate::domain::models::{Crypto, WalletAllocation};
use crate::domain::repository::HistoryRepo;
use crate::infra::coinmarketcap::MockCryptoProvider;
use crate::{AppState, api_allocations, api_history};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::routing::get;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tower::ServiceExt;

/// A fake `AllocationStore` that returns fixed BARCA targets instead of
/// reading a CSV file, so this test doesn't depend on repo-root file state.
struct FakeAllocationStore {
    targets: HashMap<String, f64>,
}

impl AllocationStore for FakeAllocationStore {
    fn read_wallet_allocations(
        &self,
        _path: &str,
    ) -> Result<Vec<WalletAllocation>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }

    fn read_barca_allocations(
        &self,
        _path: &str,
        _current_market: &str,
    ) -> Result<HashMap<String, f64>, Box<dyn Error + Send + Sync>> {
        Ok(self.targets.clone())
    }
}

async fn body_json(res: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn allocations_and_history_round_trip_through_the_real_router() {
    unsafe {
        std::env::set_var("API_KEY", "test");
        std::env::remove_var("CURRENT_MARKET");
    }

    let crypto = Crypto {
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
            created_at: None,
        },
    ];
    for wa in wallet_rows {
        repo.insert_wallet_allocation(&wa).await.unwrap();
    }

    let provider = Arc::new(MockCryptoProvider::new(vec![crypto]));
    let allocation_store = Arc::new(FakeAllocationStore {
        targets: HashMap::from([("Base".to_string(), 100.0)]),
    });
    let app_state = AppState {
        provider,
        history_repo: Arc::new(repo),
        allocation_store,
    };

    let app = Router::new()
        .route("/api/allocations", get(api_allocations))
        .route("/api/history", get(api_history))
        .with_state(app_state);

    let req = Request::builder()
        .uri("/api/allocations")
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
        .body(Body::empty())
        .unwrap();
    let totals_res = app.clone().oneshot(req_totals).await.unwrap();
    assert!(totals_res.status().is_success());
    let totals_json = body_json(totals_res).await;
    assert!(!totals_json["rows"].as_array().unwrap().is_empty());

    let req_assets = Request::builder()
        .uri("/api/history?level=assets")
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
        .body(Body::empty())
        .unwrap();
    let groups_res = app.oneshot(req_groups).await.unwrap();
    assert!(groups_res.status().is_success());
}
