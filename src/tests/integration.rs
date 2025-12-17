#[cfg(test)]
mod integration {
    use super::*;
    use tokio;
    use std::sync::Arc;
    use crate::api_client::MockCryptoProvider;
    use axum::Router;
    use tower::ServiceExt; // for `oneshot`
    use hyper::Request;
    use std::fs;

    #[tokio::test]
    async fn test_history_write_and_read_with_mock_provider() {
        // prepare temp files
        let dir = std::env::temp_dir();
        let assets_path = dir.join("hm_assets.csv");
        let barca_path = dir.join("hm_barca.csv");
        let totals_path = dir.join("hm_totals.csv");
        let assets_path_s = assets_path.to_string_lossy().to_string();
        let barca_path_s = barca_path.to_string_lossy().to_string();
        let totals_path_s = totals_path.to_string_lossy().to_string();

        // ensure clean
        let _ = fs::remove_file(&assets_path);
        let _ = fs::remove_file(&barca_path);
        let _ = fs::remove_file(&totals_path);

        // create a wallet_allocations.csv in temp dir and chdir
        let wa_path = dir.join("wallet_allocations.csv");
        let wa_content = "symbol,group,barca,target_percent,current_quantity\nBTC,Core,A,50,1\n";
        fs::write(&wa_path, wa_content).unwrap();
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        // prepare mock crypto data
        let crypto = crate::CryptoData { id:1, name: "BTC".to_string(), symbol: "BTC".to_string(), cmc_rank:1, tvl_ratio: None, tvl_usd: None, quote: crate::QuoteData{ usd: crate::PriceInfo{ price: 10.0, volume_24h:0.0, percent_change_24h:0.0, percent_change_7d:0.0, market_cap:0.0, fdv:0.0, tvl: None } } };

        let provider = Arc::new(MockCryptoProvider::new(vec![crypto]));
        let app_state = AppState{ provider, history_assets: assets_path_s.clone(), history_barca: barca_path_s.clone(), history_totals: totals_path_s.clone() };

        let app = Router::new().route("/api/allocations", get(api_allocations)).route("/api/history", get(api_history)).with_state(app_state);

        // call allocations endpoint
        let req = Request::builder().uri("/api/allocations").body(hyper::Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert!(res.status().is_success());

        // call history totals
        let req2 = Request::builder().uri("/api/history?level=totals").body(hyper::Body::empty()).unwrap();
        let res2 = app.oneshot(req2).await.unwrap();
        assert!(res2.status().is_success());
        let body_bytes = hyper::body::to_bytes(res2.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let rows = v.get("rows").and_then(|r| r.as_array()).unwrap();
        assert!(!rows.is_empty());

        // cleanup
        let _ = fs::remove_file(&assets_path);
        let _ = fs::remove_file(&barca_path);
        let _ = fs::remove_file(&totals_path);
        let _ = fs::remove_file(&wa_path);
        std::env::set_current_dir(&cwd).unwrap();
    }
}
