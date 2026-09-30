-- 0011_add_position_source_count.sql
--
-- Adds `source_count` to wallet_allocations_current: how many ledger
-- sources (distinct `notes` values) were summed into a position.
--
-- The UI used to guess "single source" by looking for " | " in the joined
-- notes, but GROUP_CONCAT skips NULLs — a position with one NULL-notes
-- source plus one named source looked single-source, and "correcting" it
-- would have written the displayed total into just one of the sources,
-- doubling the holding. Counting the sources in SQL removes the guess.
--
-- Also drops the always-NULL `id`/`created_at` placeholder columns.
DROP VIEW IF EXISTS wallet_allocations_current;
CREATE VIEW wallet_allocations_current AS
WITH per_source AS (
    SELECT
        symbol,
        group_name,
        barca,
        asset_class,
        notes,
        COALESCE(current_quantity, 0.0) AS current_quantity,
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
        GROUP_CONCAT(notes, ' | ') AS notes,
        COUNT(*) AS source_count
    FROM per_source
    WHERE rn = 1
    GROUP BY symbol, group_name, barca, asset_class
)
SELECT
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
    ON pt.symbol = q.symbol
    AND pt.group_name = COALESCE(q.group_name, '')
    AND pt.barca = COALESCE(q.barca, '')
    AND pt.asset_class = q.asset_class;
