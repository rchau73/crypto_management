-- 0006_editable_targets_and_asset_classes.sql
-- Portfolio and BARCA targets become DB-authoritative (editable via UI, with
-- a full audit trail) instead of CSV-authoritative. CSV import remains only
-- a one-time seed for brand-new entries that don't already exist in the DB.
--
-- Also introduces `asset_class` so a portfolio can hold more than crypto:
-- crypto (BullMarket/BearMarket cycle targets, as today), br-equities
-- (B3 stocks/FIIs via brapi), us-indices (via Finnhub). `market` keeps its
-- existing Bull/Bear meaning for crypto only; other asset classes don't use
-- a market cycle, so it's nullable for them.

ALTER TABLE wallet_allocations ADD COLUMN asset_class TEXT NOT NULL DEFAULT 'crypto';

-- wallet_allocations_current must partition/group by asset_class too, or a
-- br-equities row and a crypto row that happened to share a symbol/group/
-- barca would get their quantities silently summed together.
DROP VIEW IF EXISTS wallet_allocations_current;
CREATE VIEW wallet_allocations_current AS
WITH ranked AS (
    SELECT
        id,
        symbol,
        group_name,
        barca,
        target_percent,
        current_quantity,
        last_price,
        notes,
        asset_class,
        created_at,
        ROW_NUMBER() OVER (
            PARTITION BY symbol, group_name, barca, asset_class, COALESCE(notes, '')
            ORDER BY created_at DESC, id DESC
        ) AS rn
    FROM wallet_allocations
),
latest AS (
    SELECT
        symbol,
        group_name,
        barca,
        COALESCE(target_percent, 0) AS target_percent,
        COALESCE(current_quantity, 0) AS current_quantity,
        last_price,
        notes,
        asset_class,
        created_at
    FROM ranked
    WHERE rn = 1
),
aggregated AS (
    SELECT
        NULL AS id,
        symbol,
        group_name,
        barca,
        asset_class,
        MAX(target_percent) AS target_percent,
        SUM(current_quantity) AS current_quantity,
        MAX(last_price) AS last_price,
        GROUP_CONCAT(notes, ' | ') AS notes,
        MAX(created_at) AS created_at
    FROM latest
    GROUP BY symbol, group_name, barca, asset_class
)
SELECT
    id,
    symbol,
    group_name,
    barca,
    target_percent,
    current_quantity,
    last_price,
    notes,
    asset_class,
    created_at
FROM aggregated;

-- A mutable (not append-only) table of target percentages, one row per
-- (market, barca). `barca` is a top-level bucket name exactly like
-- wallet_allocations.barca (e.g. "Base", "Altcoins", and now "IBOVE" for
-- the Brazilian market) — NOT the same thing as wallet_allocations.group_name,
-- which is an orthogonal sub-grouping (e.g. "FII") within a barca. A new
-- asset class (br-equities, us-indices) is just another barca value in this
-- same flat list, not a separate scope.
--
-- `market` matches the pre-existing Bull/Bear crypto-cycle profile concept
-- (see the old wallet_barca.csv, which was matched by exact `market` string
-- with no cross-market fallback) — every barca target belongs to exactly
-- one market profile, and the full set of rows sharing a market is what
-- must sum to 100% for that profile. A barca that has nothing to do with
-- the crypto cycle (e.g. "IBOVE") is simply entered under whichever
-- market(s) a manager wants it active in; there is no automatic merging
-- across markets, matching how "Base"/"Altcoins" already require a
-- separate row per market today.
-- `market` is NOT NULL — SQLite's UNIQUE treats every NULL as distinct,
-- which would silently let duplicate (market, barca) rows through.
CREATE TABLE IF NOT EXISTS barca_targets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  market TEXT NOT NULL,
  barca TEXT NOT NULL,
  target_percent REAL NOT NULL,
  updated_by INTEGER REFERENCES users(id),
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(market, barca)
);
