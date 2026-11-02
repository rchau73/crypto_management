-- 0013_multiple_portfolios.sql
--
-- Several separate wallets ("portfolios") in one database, e.g. your own
-- plus a relative's you manage. Nothing is shared between them: every
-- wallet/target/history table gets a portfolio_id, and every UNIQUE key and
-- view now starts with it.
--
-- - portfolios: id + display name.
-- - portfolio_members: who may open which portfolio, with 'manage' (edit)
--   or 'view' (read-only) access. A user who isn't a member of a portfolio
--   can't see it at all (the API answers 404).
--
-- portfolio_id is NOT NULL with no DEFAULT on purpose: a query that forgets
-- it fails instead of silently writing into portfolio 1.
--
-- Everything that exists today becomes portfolio 1, and every existing user
-- keeps the access they had: admin/manager -> manage, user -> view.
--
-- SQLite can't add a NOT NULL column without a default or change a UNIQUE
-- key in place, so each table is rebuilt: create *_new, copy, drop, rename.
-- The views that read these tables are dropped first and recreated at the
-- end, partitioned by portfolio.

CREATE TABLE portfolios (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
INSERT INTO portfolios (id, name) VALUES (1, 'My portfolio');

CREATE TABLE portfolio_members (
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  user_id INTEGER NOT NULL REFERENCES users(id),
  access TEXT NOT NULL CHECK (access IN ('manage', 'view')),
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  PRIMARY KEY (portfolio_id, user_id)
);
CREATE INDEX idx_portfolio_members_user ON portfolio_members(user_id);
INSERT INTO portfolio_members (portfolio_id, user_id, access)
SELECT 1, id, CASE WHEN role = 'user' THEN 'view' ELSE 'manage' END FROM users;

DROP VIEW IF EXISTS wallet_allocations_current;
DROP VIEW IF EXISTS asset_variance_history;
DROP VIEW IF EXISTS group_variance_history;
DROP VIEW IF EXISTS barca_variance_history;

-- wallet_allocations (the append-only ledger) -------------------------------
CREATE TABLE wallet_allocations_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  symbol TEXT NOT NULL,
  group_name TEXT,
  barca TEXT,
  target_percent REAL,
  current_quantity REAL,
  last_price REAL,
  notes TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  asset_class TEXT NOT NULL DEFAULT 'crypto'
);
INSERT INTO wallet_allocations_new
  (id, portfolio_id, symbol, group_name, barca, target_percent, current_quantity,
   last_price, notes, created_at, asset_class)
SELECT id, 1, symbol, group_name, barca, target_percent, current_quantity,
       last_price, notes, created_at, asset_class
FROM wallet_allocations;
DROP TABLE wallet_allocations;
ALTER TABLE wallet_allocations_new RENAME TO wallet_allocations;
CREATE INDEX idx_wallet_allocations_portfolio_symbol
  ON wallet_allocations(portfolio_id, symbol, created_at);

-- portfolio_targets -----------------------------------------------------------
CREATE TABLE portfolio_targets_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  symbol TEXT NOT NULL,
  -- '' instead of NULL: SQLite's UNIQUE treats every NULL as distinct.
  group_name TEXT NOT NULL DEFAULT '',
  barca TEXT NOT NULL DEFAULT '',
  asset_class TEXT NOT NULL,
  target_percent REAL NOT NULL,
  updated_by INTEGER REFERENCES users(id),
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(portfolio_id, symbol, group_name, barca, asset_class)
);
INSERT INTO portfolio_targets_new
  (id, portfolio_id, symbol, group_name, barca, asset_class, target_percent,
   updated_by, created_at, updated_at)
SELECT id, 1, symbol, group_name, barca, asset_class, target_percent,
       updated_by, created_at, updated_at
FROM portfolio_targets;
DROP TABLE portfolio_targets;
ALTER TABLE portfolio_targets_new RENAME TO portfolio_targets;

-- barca_targets ---------------------------------------------------------------
CREATE TABLE barca_targets_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  market TEXT NOT NULL,
  barca TEXT NOT NULL,
  target_percent REAL NOT NULL,
  updated_by INTEGER REFERENCES users(id),
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(portfolio_id, market, barca)
);
INSERT INTO barca_targets_new
  (id, portfolio_id, market, barca, target_percent, updated_by, created_at, updated_at)
SELECT id, 1, market, barca, target_percent, updated_by, created_at, updated_at
FROM barca_targets;
DROP TABLE barca_targets;
ALTER TABLE barca_targets_new RENAME TO barca_targets;

-- history_assets --------------------------------------------------------------
CREATE TABLE history_assets_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
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
  UNIQUE(portfolio_id, timestamp, symbol, group_name, barca)
);
INSERT INTO history_assets_new
  (id, portfolio_id, timestamp, symbol, group_name, barca, price, current_quantity, value,
   target_percent, current_percent, market_cap, fdv, volume_24h, percent_change_24h,
   percent_change_7d, extra, created_at)
SELECT id, 1, timestamp, symbol, group_name, barca, price, current_quantity, value,
       target_percent, current_percent, market_cap, fdv, volume_24h, percent_change_24h,
       percent_change_7d, extra, created_at
FROM history_assets;
DROP TABLE history_assets;
ALTER TABLE history_assets_new RENAME TO history_assets;
CREATE INDEX idx_history_assets_portfolio_timestamp ON history_assets(portfolio_id, timestamp);

-- history_groups --------------------------------------------------------------
CREATE TABLE history_groups_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  timestamp TEXT NOT NULL,
  group_name TEXT NOT NULL,
  value REAL,
  current_percent REAL,
  target_percent REAL,
  extra TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(portfolio_id, timestamp, group_name)
);
INSERT INTO history_groups_new
  (id, portfolio_id, timestamp, group_name, value, current_percent, target_percent, extra, created_at)
SELECT id, 1, timestamp, group_name, value, current_percent, target_percent, extra, created_at
FROM history_groups;
DROP TABLE history_groups;
ALTER TABLE history_groups_new RENAME TO history_groups;
CREATE INDEX idx_history_groups_portfolio_timestamp ON history_groups(portfolio_id, timestamp);

-- history_barca ---------------------------------------------------------------
CREATE TABLE history_barca_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  timestamp TEXT NOT NULL,
  barca TEXT NOT NULL,
  value REAL,
  current_percent REAL,
  target_percent REAL,
  extra TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(portfolio_id, timestamp, barca)
);
INSERT INTO history_barca_new
  (id, portfolio_id, timestamp, barca, value, current_percent, target_percent, extra, created_at)
SELECT id, 1, timestamp, barca, value, current_percent, target_percent, extra, created_at
FROM history_barca;
DROP TABLE history_barca;
ALTER TABLE history_barca_new RENAME TO history_barca;
CREATE INDEX idx_history_barca_portfolio_timestamp ON history_barca(portfolio_id, timestamp);

-- history_totals --------------------------------------------------------------
CREATE TABLE history_totals_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  timestamp TEXT NOT NULL,
  total_value REAL,
  extra TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(portfolio_id, timestamp)
);
INSERT INTO history_totals_new (id, portfolio_id, timestamp, total_value, extra, created_at)
SELECT id, 1, timestamp, total_value, extra, created_at FROM history_totals;
DROP TABLE history_totals;
ALTER TABLE history_totals_new RENAME TO history_totals;

-- allocations (full report JSON per snapshot) ---------------------------------
CREATE TABLE allocations_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  computed_at TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO allocations_new (id, portfolio_id, computed_at, payload, created_at)
SELECT id, 1, computed_at, payload, created_at FROM allocations;
DROP TABLE allocations;
ALTER TABLE allocations_new RENAME TO allocations;

-- Views, now per portfolio ----------------------------------------------------
-- Same rules as migration 0011, with portfolio_id leading every partition,
-- GROUP BY and join.
CREATE VIEW wallet_allocations_current AS
WITH per_source AS (
    SELECT
        portfolio_id,
        symbol,
        group_name,
        barca,
        asset_class,
        notes,
        COALESCE(current_quantity, 0.0) AS current_quantity,
        last_price,
        ROW_NUMBER() OVER (
            PARTITION BY portfolio_id, symbol, group_name, barca, asset_class, COALESCE(notes, '')
            ORDER BY created_at DESC, id DESC
        ) AS rn
    FROM wallet_allocations
),
quantities AS (
    SELECT
        portfolio_id,
        symbol,
        group_name,
        barca,
        asset_class,
        SUM(current_quantity) AS current_quantity,
        MAX(last_price) AS last_price,
        GROUP_CONCAT(notes, ' | ') AS notes,
        COUNT(*) AS source_count
    FROM per_source
    WHERE rn = 1
    GROUP BY portfolio_id, symbol, group_name, barca, asset_class
)
SELECT
    q.portfolio_id,
    q.symbol,
    q.group_name,
    q.barca,
    q.asset_class,
    COALESCE(pt.target_percent, 0.0) AS target_percent,
    q.current_quantity,
    q.last_price,
    q.notes,
    q.source_count
FROM quantities q
LEFT JOIN portfolio_targets pt
    ON pt.portfolio_id = q.portfolio_id
    AND pt.symbol = q.symbol
    AND pt.group_name = COALESCE(q.group_name, '')
    AND pt.barca = COALESCE(q.barca, '')
    AND pt.asset_class = q.asset_class;

CREATE VIEW asset_variance_history AS
SELECT
  ha.portfolio_id,
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
LEFT JOIN history_totals ht
  ON ht.portfolio_id = ha.portfolio_id AND ht.timestamp = ha.timestamp;

CREATE VIEW group_variance_history AS
SELECT
  hg.portfolio_id,
  hg.timestamp,
  hg.group_name,
  hg.value,
  hg.current_percent,
  hg.target_percent,
  (hg.current_percent - hg.target_percent) AS deviation_percent
FROM history_groups hg;

CREATE VIEW barca_variance_history AS
SELECT
  hb.portfolio_id,
  hb.timestamp,
  hb.barca,
  hb.value,
  hb.current_percent,
  hb.target_percent,
  (hb.current_percent - hb.target_percent) AS deviation_percent
FROM history_barca hb;
