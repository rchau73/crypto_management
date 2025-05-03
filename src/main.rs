use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use tokio;
use dotenv::dotenv;
use std::error::Error;
use std::fs::File;
use csv::Writer;
use std::collections::HashMap;
use csv::Reader;
use axum::{
    routing::get,
    Router,
    response::Json,
};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::cors::{self, Any, CorsLayer};

// Define the structure of the API response
#[derive(Deserialize, Serialize, Debug)]
struct ApiResponse {
    status: ApiStatus,
    data: Vec<CryptoData>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ApiStatus {
    timestamp: String,
    error_code: i32,
    error_message: Option<String>,
    credit_count: i32,
    notice: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct CryptoData {
    id: u32,
    name: String,
    symbol: String,
    tvl_ratio: Option<f64>,
    tvl_usd: Option<f64>,
    quote: QuoteData,
}

#[derive(Deserialize, Serialize, Debug)]
struct QuoteData {
    #[serde(rename = "USD")]
    usd: PriceInfo,
}

#[derive(Deserialize, Serialize, Debug)]
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
    target_percent: f64,
    current_quantity: f64,
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
    println!("\n************************************");
    println!("Response: {:?}", parsed_response.status);
    println!("************************************\n\n");
    Ok(parsed_response.data)
}

// Read wallet allocations from CSV
fn read_wallet_allocations(path: &str) -> Result<Vec<WalletAllocation>, Box<dyn Error>> {
    let mut rdr = Reader::from_path(path)?;
    let mut allocations = Vec::new();
    for result in rdr.deserialize() {
        let record: WalletAllocation = result?;
        allocations.push(record);
    }
    Ok(allocations)
}

async fn api_allocations() -> Json<serde_json::Value> {
    dotenv().ok();
    let api_key = std::env::var("API_KEY").unwrap();
    let allocations = read_wallet_allocations("wallet_allocations.csv").unwrap();
    let cryptos = fetch_crypto_data(&api_key).await.unwrap();

    let crypto_map: HashMap<String, &CryptoData> = cryptos.iter()
        .map(|c| (c.symbol.clone(), c))
        .collect();

    let mut total_wallet_value = 0.0;
    let mut asset_values = HashMap::new();
    for alloc in &allocations {
        if let Some(crypto) = crypto_map.get(&alloc.symbol) {
            let price = crypto.quote.usd.price;
            let value = alloc.current_quantity * price;
            asset_values.insert(&alloc.symbol, value);
            total_wallet_value += value;
        }
    }

    let mut group_targets: HashMap<String, f64> = HashMap::new();
    let mut group_values: HashMap<String, f64> = HashMap::new();
    for alloc in &allocations {
        let group = &alloc.group;
        let value = asset_values.get(&alloc.symbol).copied().unwrap_or(0.0);
        *group_targets.entry(group.clone()).or_insert(0.0) += alloc.target_percent;
        *group_values.entry(group.clone()).or_insert(0.0) += value;
    }

    let per_asset: Vec<_> = allocations.iter().map(|alloc| {
        let price = crypto_map.get(&alloc.symbol).map(|c| c.quote.usd.price).unwrap_or(0.0);
        let value = asset_values.get(&alloc.symbol).copied().unwrap_or(0.0);
        let current_percent = if total_wallet_value > 0.0 {
            (value / total_wallet_value) * 100.0
        } else {
            0.0
        };
        let deviation = current_percent - alloc.target_percent;
        json!({
            "symbol": alloc.symbol,
            "group": alloc.group,
            "price": price,
            "current_quantity": alloc.current_quantity,
            "value": value,
            "target_percent": alloc.target_percent,
            "current_percent": current_percent,
            "deviation": deviation
        })
    }).collect();

    let per_group: Vec<_> = group_targets.iter().map(|(group, group_target)| {
        let group_value = group_values.get(group).copied().unwrap_or(0.0);
        let group_percent = if total_wallet_value > 0.0 {
            (group_value / total_wallet_value) * 100.0
        } else {
            0.0
        };
        let deviation = group_percent - group_target;
        json!({
            "group": group,
            "target_percent": group_target,
            "current_percent": group_percent,
            "deviation": deviation
        })
    }).collect();

    Json(json!({
        "per_asset": per_asset,
        "per_group": per_group
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let backend = async {
            let app = Router::new()
        .route("/api/allocations", get(api_allocations))
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
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap();
    axum::serve(listener, app).await;
}