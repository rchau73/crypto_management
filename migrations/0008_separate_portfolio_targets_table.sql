-- 0008_separate_portfolio_targets_table.sql
--
-- 0007 tried to fix target_percent aggregation by taking the single most
-- recently-inserted wallet_allocations row overall (regardless of which
-- notes/source partition it belonged to). That's still broken: a symbol
-- split across multiple wallets/exchanges gets one ledger row per source,
-- and for data that was never edited through the Portfolio Targets admin
-- UI, "most recently inserted" reflects arbitrary CSV row order within an
-- import batch, not intent — a freshly-imported multi-source asset could
-- show the wrong (often 0) target depending on which of its source rows
-- happened to be inserted last.
--
-- The real problem: target_percent is a single, asset-level configuration
-- value, not a per-source ledger fact — it doesn't belong interleaved with
-- append-only quantity history at all. `barca_targets` already uses the
-- right pattern for this (a small mutable table, not a ledger); this moves
-- portfolio targets to the same pattern.
CREATE TABLE IF NOT EXISTS portfolio_targets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  symbol TEXT NOT NULL,
  -- '' sentinel instead of NULL, same reasoning as barca_targets.market —
  -- SQLite's UNIQUE treats every NULL as distinct, which would silently
  -- allow duplicate rows for any asset with an absent group/barca.
  group_name TEXT NOT NULL DEFAULT '',
  barca TEXT NOT NULL DEFAULT '',
  asset_class TEXT NOT NULL,
  target_percent REAL NOT NULL,
  updated_by INTEGER REFERENCES users(id),
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(symbol, group_name, barca, asset_class)
);

-- Backfill from existing history: MAX(target_percent) across every ledger
-- row for a key is the right one-time migration read (unlike ongoing
-- aggregation, a one-time backfill only needs to find "the value it was
-- ever intentionally set to", and historically only one source partition
-- per key ever carried a non-zero value).
INSERT INTO portfolio_targets (symbol, group_name, barca, asset_class, target_percent)
SELECT
    symbol,
    COALESCE(group_name, ''),
    COALESCE(barca, ''),
    asset_class,
    MAX(COALESCE(target_percent, 0.0))
FROM wallet_allocations
GROUP BY symbol, COALESCE(group_name, ''), COALESCE(barca, ''), asset_class;

-- wallet_allocations_current now only aggregates quantity/notes (its
-- original, correct job — sum per-source quantities, concat their notes)
-- and reads target_percent from the new authoritative table.
DROP VIEW IF EXISTS wallet_allocations_current;
CREATE VIEW wallet_allocations_current AS
WITH per_source AS (
    SELECT
        symbol,
        group_name,
        barca,
        asset_class,
        notes,
        COALESCE(current_quantity, 0) AS current_quantity,
        last_price,
        ROW_NUMBER() OVER (
            PARTITION BY symbol, group_name, barca, asset_class, COALESCE(notes, '')
            ORDER BY created_at DESC, id DESC
        ) AS rn
    FROM wallet_allocations
),
quantities AS (
    SELECT
        symbol,
        group_name,
        barca,
        asset_class,
        SUM(current_quantity) AS current_quantity,
        MAX(last_price) AS last_price,
        GROUP_CONCAT(notes, ' | ') AS notes
    FROM per_source
    WHERE rn = 1
    GROUP BY symbol, group_name, barca, asset_class
)
SELECT
    NULL AS id,
    q.symbol,
    q.group_name,
    q.barca,
    COALESCE(pt.target_percent, 0.0) AS target_percent,
    q.current_quantity,
    q.last_price,
    q.notes,
    q.asset_class,
    NULL AS created_at
FROM quantities q
LEFT JOIN portfolio_targets pt
    ON pt.symbol = q.symbol
    AND pt.group_name = COALESCE(q.group_name, '')
    AND pt.barca = COALESCE(q.barca, '')
    AND pt.asset_class = q.asset_class;
