-- 0001_create_history_tables.sql
-- Create history tables for assets, barca, totals, wallet allocations and computed allocations

CREATE TABLE IF NOT EXISTS history_assets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp TEXT NOT NULL,
  symbol TEXT NOT NULL,
  group_name TEXT,
  barca TEXT,
  price REAL,
  current_quantity REAL,
  value REAL,
  target_percent REAL,
  current_percent REAL,
  market_cap REAL,
  fdv REAL,
  volume_24h REAL,
  percent_change_24h REAL,
  percent_change_7d REAL,
  extra TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(timestamp, symbol)
);
CREATE INDEX IF NOT EXISTS idx_history_assets_timestamp ON history_assets(timestamp);

CREATE TABLE IF NOT EXISTS history_barca (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp TEXT NOT NULL,
  barca TEXT NOT NULL,
  value REAL,
  current_percent REAL,
  target_percent REAL,
  extra TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(timestamp, barca)
);
CREATE INDEX IF NOT EXISTS idx_history_barca_timestamp ON history_barca(timestamp);

CREATE TABLE IF NOT EXISTS history_totals (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp TEXT NOT NULL,
  total_value REAL,
  extra TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(timestamp)
);
CREATE INDEX IF NOT EXISTS idx_history_totals_timestamp ON history_totals(timestamp);

-- wallet_allocations maps static allocation targets (imported from wallet_allocations.csv)
-- wallet_allocations is append-only for audit: every change is a new row.
CREATE TABLE IF NOT EXISTS wallet_allocations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  symbol TEXT NOT NULL,
  group_name TEXT,
  barca TEXT,
  target_percent REAL,
  current_quantity REAL,
  last_price REAL,
  notes TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_wallet_allocations_symbol_created_at ON wallet_allocations(symbol, created_at);

-- Convenience view to show the latest wallet allocation per symbol (read-only)
CREATE VIEW IF NOT EXISTS wallet_allocations_current AS
SELECT wa.* FROM wallet_allocations wa
JOIN (
  SELECT symbol, MAX(created_at) AS max_created
  FROM wallet_allocations
  GROUP BY symbol
) latest ON latest.symbol = wa.symbol AND latest.max_created = wa.created_at;

-- allocations: optional persisted computed allocations (JSON payload)
CREATE TABLE IF NOT EXISTS allocations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  computed_at TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
