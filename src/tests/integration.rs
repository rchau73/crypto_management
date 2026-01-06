#[cfg(test)]
mod integration {
    use super::*;
    use tokio;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use crate::api_client::MockCryptoProvider;
    use axum::Router;
    use tower::ServiceExt; // for `oneshot`
    use hyper::Request;
    use crate::domain::models::WalletAllocation;

    #[tokio::test]
    async fn test_history_write_and_read_with_mock_provider() {
        std::env::set_var("API_KEY", "test");
        std::env::remove_var("CURRENT_MARKET");

        let crypto = crate::CryptoData {
            id: 1,
            name: "BTC".to_string(),
            symbol: "BTC".to_string(),
            cmc_rank: 1,
            tvl_ratio: None,
            tvl_usd: None,
            quote: crate::QuoteData {
                usd: crate::PriceInfo {
                    price: 10.0,
                    volume_24h: 0.0,
                    percent_change_24h: 0.0,
                    percent_change_7d: 0.0,
                    market_cap: 0.0,
                    fdv: 0.0,
                    tvl: None,
                },
            },
        };

        let pool = SqlitePool::connect("sqlite::memory:").await.expect("failed to connect to in-memory db");
        sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
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
        let app_state = AppState { provider, history_repo: std::sync::Arc::new(repo) };

        let app = Router::new()
            .route("/api/allocations", get(api_allocations))
            .route("/api/history", get(api_history))
            .with_state(app_state);

        let req = Request::builder().uri("/api/allocations").body(hyper::Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert!(res.status().is_success());

        let req_totals = Request::builder().uri("/api/history?level=totals").body(hyper::Body::empty()).unwrap();
        let totals_res = app.clone().oneshot(req_totals).await.unwrap();
        assert!(totals_res.status().is_success());
        let totals_body = hyper::body::to_bytes(totals_res.into_body()).await.unwrap();
        let totals_json: serde_json::Value = serde_json::from_slice(&totals_body).unwrap();
        assert!(totals_json["rows"].as_array().unwrap().len() >= 1);

        let req_assets = Request::builder().uri("/api/history?level=assets").body(hyper::Body::empty()).unwrap();
        let assets_res = app.clone().oneshot(req_assets).await.unwrap();
        assert!(assets_res.status().is_success());
        let assets_body = hyper::body::to_bytes(assets_res.into_body()).await.unwrap();
        let assets_json: serde_json::Value = serde_json::from_slice(&assets_body).unwrap();
        let asset_rows = assets_json["rows"].as_array().unwrap();
        assert!(!asset_rows.is_empty());
        let first = &asset_rows[0];
        assert!(first.get("value_deviation").is_some());

        let req_groups = Request::builder().uri("/api/history?level=groups").body(hyper::Body::empty()).unwrap();
        let groups_res = app.oneshot(req_groups).await.unwrap();
        assert!(groups_res.status().is_success());
    }
}
