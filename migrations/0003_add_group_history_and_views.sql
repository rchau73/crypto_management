-- 0003_add_group_history_and_views.sql
-- Add history_groups table plus variance views for dashboard queries.

CREATE TABLE IF NOT EXISTS history_groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp TEXT NOT NULL,
  group_name TEXT NOT NULL,
  value REAL,
  current_percent REAL,
  target_percent REAL,
  extra TEXT,
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(timestamp, group_name)
);
CREATE INDEX IF NOT EXISTS idx_history_groups_timestamp ON history_groups(timestamp);

DROP VIEW IF EXISTS asset_variance_history;
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

DROP VIEW IF EXISTS barca_variance_history;
CREATE VIEW barca_variance_history AS
SELECT
  hb.timestamp,
  hb.barca,
  hb.value,
  hb.current_percent,
  hb.target_percent,
  (hb.current_percent - hb.target_percent) AS deviation_percent
FROM history_barca hb;

DROP VIEW IF EXISTS group_variance_history;
CREATE VIEW group_variance_history AS
SELECT
  hg.timestamp,
  hg.group_name,
  hg.value,
  hg.current_percent,
  hg.target_percent,
  (hg.current_percent - hg.target_percent) AS deviation_percent
FROM history_groups hg;
