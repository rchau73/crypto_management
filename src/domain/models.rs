use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single asset's current market data, as needed by the allocation use
/// case — crypto, Brazilian B3/FIIs, or a US index, whichever `AssetClass`
/// it belongs to. Deliberately independent of any external API's wire
/// format — `infra::coinmarketcap`/`infra::brapi`/`infra::finnhub` each map
/// their own provider's response shape into this. `market_cap`/`fdv`/
/// `volume_24h`/`percent_change_*` are CoinMarketCap-specific extras with no
/// equivalent from brapi/Finnhub's quote endpoints, so non-crypto providers
/// just leave them at 0.0 — nothing downstream (`compute_allocations`)
/// reads them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketQuote {
    pub symbol: String,
    pub price: f64,
    pub market_cap: f64,
    pub fdv: f64,
    pub volume_24h: f64,
    pub percent_change_24h: f64,
    pub percent_change_7d: f64,
}

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
    // "crypto" | "br-equities" | "us-indices" — see AssetClass. Defaults to
    // "crypto" both at the DB level (existing rows didn't need a backfill
    // script) and here (a CSV row / API payload that omits it is assumed crypto).
    #[serde(default = "default_asset_class")]
    pub asset_class: String,
    pub created_at: Option<String>,
}

fn default_asset_class() -> String {
    AssetClass::Crypto.as_str().to_string()
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

/// The three permission tiers. Ordered `User < Manager < Admin` so
/// `current_role.satisfies(minimum)` is a plain `>=` comparison — no policy
/// engine needed for three fixed tiers.
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

/// Which market this asset belongs to, for pricing (which provider to call)
/// and for target-percent grouping (BARCA). Only `Crypto` has a Bull/Bear
/// market-cycle concept — the others use `"default"` for `market`.
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

// A user account. `role` is stored as plain TEXT (validated against
// Role::parse at the usecase boundary) rather than a custom sqlx type, to
// keep this struct a trivial FromRow mapping like the rest of this module.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Option<i64>,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    // Mandatory + unique (see migrations/0005): the intended anchor for
    // future account activation / email verification / password-reset-link
    // flows, none of which are built yet — this is just the field.
    pub email: String,
    pub phone: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// Opaque, single-use-per-refresh session token (append-only; a used/expired
// row is marked revoked rather than deleted, for audit).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: Option<i64>,
    pub user_id: i64,
    pub token_hash: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub created_at: Option<String>,
}

// A target percentage for one (market, barca) — `barca` is the same
// top-level bucket concept as WalletAllocation::barca (e.g. "Base",
// "Altcoins", "IBOVE"), not the sub-grouping in WalletAllocation::group_name.
// Mutable (not append-only, unlike WalletAllocation) — an admin/manager
// editing targets is changing configuration, not recording a new ledger entry.
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

// A target percentage for one (symbol, group_name, barca, asset_class) —
// mutable configuration, not ledger history, same reasoning as BarcaTarget.
// wallet_allocations stays append-only for quantity/notes (per funding
// source, summed); target_percent lives here instead so editing it can
// never compete against a stale value left behind in some other funding
// source's ledger row (see migrations/0008).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PortfolioTarget {
    pub id: Option<i64>,
    pub symbol: String,
    pub group_name: String,
    pub barca: String,
    pub asset_class: String,
    pub target_percent: f64,
    pub updated_by: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod role_tests {
    use super::Role;

    #[test]
    fn parses_known_role_strings() {
        assert_eq!(Role::parse("admin"), Some(Role::Admin));
        assert_eq!(Role::parse("manager"), Some(Role::Manager));
        assert_eq!(Role::parse("user"), Some(Role::User));
    }

    #[test]
    fn rejects_unknown_role_strings() {
        assert_eq!(Role::parse("superuser"), None);
        assert_eq!(Role::parse(""), None);
        assert_eq!(Role::parse("Admin"), None); // case-sensitive on purpose
    }

    #[test]
    fn as_str_round_trips_through_parse() {
        for role in [Role::Admin, Role::Manager, Role::User] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
    }

    #[test]
    fn satisfies_is_a_minimum_rank_check() {
        assert!(Role::Admin.satisfies(Role::User));
        assert!(Role::Admin.satisfies(Role::Manager));
        assert!(Role::Admin.satisfies(Role::Admin));
        assert!(Role::Manager.satisfies(Role::User));
        assert!(!Role::Manager.satisfies(Role::Admin));
        assert!(!Role::User.satisfies(Role::Manager));
        assert!(Role::User.satisfies(Role::User));
    }
}
