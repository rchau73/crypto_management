-- 0007_fix_target_percent_aggregation.sql
--
-- Bug: wallet_allocations_current computed target_percent as MAX(target_percent)
-- across each asset's per-notes-source partitions (the same partitioning used
-- for current_quantity, since a symbol can have several ledger rows from
-- different wallets/exchanges sharing one symbol/group/barca/asset_class).
-- target_percent is an asset-level decision, not a per-source one — but
-- because only ONE historical source partition typically ever carried a
-- non-zero target_percent (the rest were 0), editing the target via the
-- Portfolio Targets admin UI inserts a NEW, separate notes-partition (see
-- migrations/0006's caution note) whose target_percent then had to WIN a
-- MAX() against that stale old partition's value — so a target INCREASE
-- silently failed to apply whenever the untouched partition's old value was
-- still larger, and a target DECREASE never applied at all (MAX() can only
-- go up). Observed live: reducing USDT's target 30% -> 29% did not take
-- effect, because an old "Binance" source-partition row still held 30%.
--
-- Fix: target_percent (and last_price, same reasoning) now come from the
-- single most-recently-created row for the whole (symbol, group, barca,
-- asset_class) — regardless of which notes/source partition it belongs to
-- — so the latest edit always wins outright. current_quantity/notes keep
-- the existing per-source-partition-then-summed logic (that part was
-- correct: quantities really do need to add up across wallets).
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
        GROUP_CONCAT(notes, ' | ') AS notes
    FROM per_source
    WHERE rn = 1
    GROUP BY symbol, group_name, barca, asset_class
),
per_asset AS (
    SELECT
        id,
        symbol,
        group_name,
        barca,
        asset_class,
        COALESCE(target_percent, 0) AS target_percent,
        last_price,
        created_at,
        ROW_NUMBER() OVER (
            PARTITION BY symbol, group_name, barca, asset_class
            ORDER BY created_at DESC, id DESC
        ) AS rn
    FROM wallet_allocations
)
SELECT
    pa.id,
    pa.symbol,
    pa.group_name,
    pa.barca,
    pa.target_percent,
    q.current_quantity,
    pa.last_price,
    q.notes,
    pa.asset_class,
    pa.created_at
FROM per_asset pa
JOIN quantities q
    ON q.symbol = pa.symbol
    AND q.group_name IS pa.group_name
    AND q.barca IS pa.barca
    AND q.asset_class = pa.asset_class
WHERE pa.rn = 1;
