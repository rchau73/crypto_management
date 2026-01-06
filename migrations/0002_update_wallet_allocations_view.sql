-- 0002_update_wallet_allocations_view.sql
-- Ensure wallet_allocations_current aggregates every wallet row (latest per wallet) by symbol/group/barca.

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
        created_at,
        ROW_NUMBER() OVER (
            PARTITION BY symbol, group_name, barca, COALESCE(notes, '')
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
        MAX(target_percent) AS target_percent,
        SUM(current_quantity) AS current_quantity,
        MAX(last_price) AS last_price,
        GROUP_CONCAT(notes, ' | ') AS notes,
        MAX(created_at) AS created_at
    FROM latest
    GROUP BY symbol, group_name, barca
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
    created_at
FROM aggregated;
