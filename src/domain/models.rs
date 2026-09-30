//! Plain data shared by every layer. No I/O and no business rules live
//! here — just the shapes of things, plus tiny parse/format helpers for the
//! two enums.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---------------------------------------------------------------------------
// Market data
// ---------------------------------------------------------------------------

/// The current price of one asset, whichever provider it came from
/// (CoinMarketCap, brapi or Finnhub). Each `infra` adapter maps its own
/// wire format into this, so nothing past the adapter knows which API
/// answered.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketQuote {
    pub symbol: String,
    pub price: f64,
}

/// Which market an asset belongs to — decides which provider prices it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetClass {
    Crypto,
    BrEquities,
    UsIndices,
}

impl AssetClass {
    pub fn parse(s: &str) -> Option<AssetClass> {
        match s {
            "crypto" => Some(AssetClass::Crypto),
            "br-equities" => Some(AssetClass::BrEquities),
            "us-indices" => Some(AssetClass::UsIndices),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AssetClass::Crypto => "crypto",
            AssetClass::BrEquities => "br-equities",
            AssetClass::UsIndices => "us-indices",
        }
    }
}

// ---------------------------------------------------------------------------
// Portfolio: the append-only ledger and the positions derived from it
// ---------------------------------------------------------------------------

/// Identifies one displayed portfolio row. Missing group/barca are stored
/// as NULL in the ledger but compared as "" everywhere, matching the
/// `COALESCE(..., '')` in the `wallet_allocations_current` view.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PositionKey {
    pub symbol: String,
    pub group_name: String,
    pub barca: String,
    pub asset_class: String,
}

/// One row written to the append-only `wallet_allocations` ledger. Rows are
/// never updated or deleted: a newer row for the same source (same key +
/// same `notes`) supersedes the older one.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerEntry {
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub asset_class: String,
    pub current_quantity: f64,
    pub last_price: Option<f64>,
    /// Names the funding source (wallet, exchange, broker...). Each distinct
    /// value is a separate source whose quantity is added to the others.
    pub notes: Option<String>,
}

/// One row of the `wallet_allocations_current` view: the latest quantity of
/// every source for a key, summed, plus that key's target percent.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct WalletPosition {
    pub symbol: String,
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub asset_class: String,
    pub target_percent: f64,
    pub current_quantity: f64,
    pub last_price: Option<f64>,
    /// Every source's notes joined with " | ".
    pub notes: Option<String>,
    /// How many ledger sources were summed into `current_quantity`. Only a
    /// single-source position can have its quantity corrected directly.
    pub source_count: i64,
}

impl WalletPosition {
    pub fn key(&self) -> PositionKey {
        PositionKey {
            symbol: self.symbol.clone(),
            group_name: self.group_name.clone().unwrap_or_default(),
            barca: self.barca.clone().unwrap_or_default(),
            asset_class: self.asset_class.clone(),
        }
    }
}

/// A target percent for one position key (the `portfolio_targets` table).
#[derive(Debug, Clone, PartialEq)]
pub struct NewPortfolioTarget {
    pub key: PositionKey,
    pub target_percent: f64,
}

// ---------------------------------------------------------------------------
// BARCA targets
// ---------------------------------------------------------------------------

/// A target percent for one top-level bucket ("barca") in one market
/// profile ("BullMarket", "BearMarket", ...).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BarcaTarget {
    pub id: Option<i64>,
    pub market: String,
    pub barca: String,
    pub target_percent: f64,
    pub updated_by: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct NewBarcaTarget {
    pub barca: String,
    pub target_percent: f64,
}

// ---------------------------------------------------------------------------
// Allocation report (the /api/allocations response)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AssetAllocation {
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GroupAllocation {
    pub group: String,
    pub value: f64,
    pub target_percent: f64,
    pub current_percent: f64,
    pub deviation: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BarcaAllocation {
    pub barca: String,
    pub value: f64,
    pub target_percent: f64,
    pub current_percent: f64,
    pub deviation: f64,
}

/// Actual value of a barca, without a target (a barca may have holdings but
/// no target, or a target but no holdings yet).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BarcaActual {
    pub barca: String,
    pub value: f64,
    pub current_percent: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct AllocationReport {
    pub total_value: f64,
    pub per_asset: Vec<AssetAllocation>,
    pub per_group: Vec<GroupAllocation>,
    pub per_barca: Vec<BarcaAllocation>,
    pub per_barca_actual: Vec<BarcaActual>,
}

// ---------------------------------------------------------------------------
// History snapshots
// ---------------------------------------------------------------------------

/// Everything recorded for one "Update Prices" click, written atomically.
#[derive(Debug, Clone)]
pub struct AllocationSnapshot {
    pub timestamp: String, // RFC 3339
    pub report: AllocationReport,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AssetHistoryRow {
    pub timestamp: String,
    pub symbol: String,
    #[serde(rename = "group")]
    pub group_name: Option<String>,
    pub barca: Option<String>,
    pub price: Option<f64>,
    pub current_quantity: Option<f64>,
    pub value: Option<f64>,
    pub target_percent: Option<f64>,
    pub current_percent: Option<f64>,
    #[serde(rename = "deviation")]
    pub deviation_percent: Option<f64>,
    pub value_deviation: Option<f64>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct GroupHistoryRow {
    pub timestamp: String,
    #[serde(rename = "group")]
    pub group_name: String,
    pub value: Option<f64>,
    pub current_percent: Option<f64>,
    pub target_percent: Option<f64>,
    #[serde(rename = "deviation")]
    pub deviation_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BarcaHistoryRow {
    pub timestamp: String,
    pub barca: String,
    pub value: Option<f64>,
    pub current_percent: Option<f64>,
    pub target_percent: Option<f64>,
    #[serde(rename = "deviation")]
    pub deviation_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TotalHistoryRow {
    pub timestamp: String,
    pub total_value: Option<f64>,
}

// ---------------------------------------------------------------------------
// Users and sessions
// ---------------------------------------------------------------------------

/// The three permission tiers. Ordered `User < Manager < Admin`, so
/// `role.satisfies(minimum)` is a plain `>=` comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Manager,
    Admin,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "admin" => Some(Role::Admin),
            "manager" => Some(Role::Manager),
            "user" => Some(Role::User),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Manager => "manager",
            Role::User => "user",
        }
    }

    pub fn satisfies(&self, minimum: Role) -> bool {
        *self >= minimum
    }
}

/// A user account. `role` is stored as plain TEXT and validated with
/// `Role::parse` at the use-case boundary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Option<i64>,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub email: String,
    pub phone: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// A stored refresh token. Only the SHA-256 hash of the token is kept; a
/// revoked or expired row is kept (marked) rather than deleted, for audit.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: Option<i64>,
    pub user_id: i64,
    pub token_hash: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub created_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_parses_known_strings_case_sensitively() {
        assert_eq!(Role::parse("admin"), Some(Role::Admin));
        assert_eq!(Role::parse("manager"), Some(Role::Manager));
        assert_eq!(Role::parse("user"), Some(Role::User));
        assert_eq!(Role::parse("superuser"), None);
        assert_eq!(Role::parse(""), None);
        assert_eq!(Role::parse("Admin"), None);
    }

    #[test]
    fn role_as_str_round_trips_through_parse() {
        for role in [Role::Admin, Role::Manager, Role::User] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
    }

    #[test]
    fn role_satisfies_is_a_minimum_rank_check() {
        assert!(Role::Admin.satisfies(Role::Manager));
        assert!(Role::Manager.satisfies(Role::User));
        assert!(Role::User.satisfies(Role::User));
        assert!(!Role::Manager.satisfies(Role::Admin));
        assert!(!Role::User.satisfies(Role::Manager));
    }

    #[test]
    fn asset_class_round_trips_and_rejects_unknown_values() {
        for class in [
            AssetClass::Crypto,
            AssetClass::BrEquities,
            AssetClass::UsIndices,
        ] {
            assert_eq!(AssetClass::parse(class.as_str()), Some(class));
        }
        assert_eq!(AssetClass::parse("stocks"), None);
    }

    #[test]
    fn position_key_treats_missing_group_and_barca_as_empty() {
        let position = WalletPosition {
            symbol: "BTC".into(),
            group_name: None,
            barca: None,
            asset_class: "crypto".into(),
            target_percent: 0.0,
            current_quantity: 1.0,
            last_price: None,
            notes: None,
            source_count: 1,
        };
        let key = position.key();
        assert_eq!(key.group_name, "");
        assert_eq!(key.barca, "");
    }
}
