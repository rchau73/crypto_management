use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub status: ApiStatus,
    pub data: Vec<CryptoData>,
}

#[derive(Debug, Deserialize)]
pub struct ApiStatus {
    pub timestamp: String,
    pub error_code: i32,
    pub error_message: Option<String>,
    pub credit_count: i32,
    pub notice: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CryptoData {
    pub id: u32,
    pub name: String,
    pub symbol: String,
    pub cmc_rank: u32,
    pub tvl_ratio: Option<f64>,
    pub tvl_usd: Option<f64>,
    pub quote: QuoteData,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuoteData {
    #[serde(rename = "USD")]
    pub usd: PriceInfo,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceInfo {
    pub price: f64,
    pub volume_24h: f64,
    pub percent_change_24h: f64,
    pub percent_change_7d: f64,
    pub market_cap: f64,
    #[serde(rename = "fully_diluted_market_cap")]
    pub fdv: f64,
    pub tvl: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct WalletAllocation {
    pub symbol: String,
    pub group: String,
    pub barca: String,
    pub target_percent: f64,
    pub current_quantity: f64,
    #[serde(default)]
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BarcaAllocation {
    pub market: String,
    pub group: String,
    pub target_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct PerAssetAllocation {
    pub symbol: String,
    pub group: String,
    pub barca: String,
    pub price: f64,
    pub current_quantity: f64,
    pub value: f64,
    pub target_percent: f64,
    pub current_percent: f64,
    pub deviation: f64,
}

#[derive(Debug, Serialize)]
pub struct PerGroupAllocation {
    pub group: String,
    pub target_percent: f64,
    pub current_percent: f64,
    pub deviation: f64,
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct PerBarcaAllocation {
    pub barca: String,
    pub target_percent: f64,
    pub current_percent: f64,
    pub deviation: f64,
}

#[derive(Debug, Serialize)]
pub struct PerBarcaActualAllocation {
    pub barca: String,
    pub value: f64,
    pub current_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct AllocationResponse {
    pub per_asset: Vec<PerAssetAllocation>,
    pub per_group: Vec<PerGroupAllocation>,
    pub per_barca: Vec<PerBarcaAllocation>,
    pub per_barca_actual: Vec<PerBarcaActualAllocation>,
}
