use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Asset snapshot row (history_assets)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AssetSnapshot {
    pub id: Option<i64>,
    pub timestamp: String, // ISO8601
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub price: Option<f64>,
    pub current_quantity: Option<f64>,
    pub value: Option<f64>,
    pub target_percent: Option<f64>,
    pub current_percent: Option<f64>,
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub volume_24h: Option<f64>,
    pub percent_change_24h: Option<f64>,
    pub percent_change_7d: Option<f64>,
    pub extra: Option<serde_json::Value>,
    pub created_at: Option<String>,
}

// BARCA snapshot row (history_barca)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BarcaSnapshot {
    pub id: Option<i64>,
    pub timestamp: String,
    pub barca: String,
    pub value: Option<f64>,
    pub current_percent: Option<f64>,
    pub target_percent: Option<f64>,
    pub extra: Option<serde_json::Value>,
    pub created_at: Option<String>,
}

// Group snapshot row (history_groups)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupSnapshot {
    pub id: Option<i64>,
    pub timestamp: String,
    pub group_name: String,
    pub value: Option<f64>,
    pub current_percent: Option<f64>,
    pub target_percent: Option<f64>,
    pub extra: Option<serde_json::Value>,
    pub created_at: Option<String>,
}

// Totals snapshot row (history_totals)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TotalSnapshot {
    pub id: Option<i64>,
    pub timestamp: String,
    pub total_value: Option<f64>,
    pub extra: Option<serde_json::Value>,
    pub created_at: Option<String>,
}

// Wallet allocation ledger (append-only) row (wallet_allocations)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WalletAllocation {
    pub id: Option<i64>,
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub target_percent: Option<f64>,
    pub current_quantity: Option<f64>,
    pub last_price: Option<f64>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}

// Persisted allocation computation (allocations)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AllocationRecord {
    pub id: Option<i64>,
    pub computed_at: String,
    pub payload: serde_json::Value,
    pub created_at: Option<String>,
}

// Read models for dashboard/history endpoints
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AssetHistoryRow {
    pub timestamp: String,
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub price: Option<f64>,
    pub current_quantity: Option<f64>,
    pub value: Option<f64>,
    pub target_percent: Option<f64>,
    pub current_percent: Option<f64>,
    pub deviation_percent: Option<f64>,
    pub value_deviation: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BarcaHistoryRow {
    pub timestamp: String,
    pub barca: String,
    pub value: Option<f64>,
    pub current_percent: Option<f64>,
    pub target_percent: Option<f64>,
    pub deviation_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupHistoryRow {
    pub timestamp: String,
    pub group_name: String,
    pub value: Option<f64>,
    pub current_percent: Option<f64>,
    pub target_percent: Option<f64>,
    pub deviation_percent: Option<f64>,
}
