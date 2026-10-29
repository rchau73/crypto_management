-- 0012_history_assets_unique_per_position.sql
--
-- history_assets was UNIQUE(timestamp, symbol), and record_snapshot writes
-- with INSERT OR IGNORE — so a symbol held in two groups (BTC in Holding and
-- DeFi, USDT, ETH...) silently kept only one row per snapshot. The key is
-- now the whole position: (timestamp, symbol, group_name, barca).
--
-- SQLite can't change a UNIQUE constraint in place: rebuild the table and
-- copy every row (the old key is stricter, so nothing can conflict). The
-- view that reads it is dropped first and recreated unchanged.

DROP VIEW IF EXISTS asset_variance_history;

CREATE TABLE history_assets_new (
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
  UNIQUE(timestamp, symbol, group_name, barca)
);

INSERT INTO history_assets_new
SELECT id, timestamp, symbol, group_name, barca, price, current_quantity, value,
       target_percent, current_percent, market_cap, fdv, volume_24h,
       percent_change_24h, percent_change_7d, extra, created_at
FROM history_assets;

DROP TABLE history_assets;
ALTER TABLE history_assets_new RENAME TO history_assets;
CREATE INDEX idx_history_assets_timestamp ON history_assets(timestamp);

CREATE VIEW asset_variance_history AS
SELECT
  ha.timestamp,
  ha.symbol,
  ha.group_name,
  ha.barca,
  ha.price,
  ha.current_quantity,
  ha.value,
  ha.target_percent,
  ha.current_percent,
  (ha.current_percent - ha.target_percent) AS deviation_percent,
  (ha.value - (COALESCE(ht.total_value, 0) * COALESCE(ha.target_percent, 0) / 100.0)) AS value_deviation
FROM history_assets ha
LEFT JOIN history_totals ht ON ht.timestamp = ha.timestamp;
