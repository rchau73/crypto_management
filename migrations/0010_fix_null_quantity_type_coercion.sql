-- 0010_fix_null_quantity_type_coercion.sql
--
-- Bug: wallet_allocations_current's per_source CTE wrote
-- `COALESCE(current_quantity, 0)` — an INTEGER literal `0`, not `0.0`. For
-- any (symbol, group, barca, asset_class, notes) partition whose only row
-- has current_quantity = NULL (e.g. a CSV row whose quantity column didn't
-- match "current_quantity" by name), SUM() over that all-substituted-zero
-- input returns SQLite's INTEGER storage class instead of REAL — which
-- sqlx's strict decode to `Option<f64>` rejects outright, crashing
-- `/api/allocations` for every symbol in that state with a "mismatched
-- types ... SQL type INTEGER" error. Observed live after uploading a CSV
-- whose quantity column was named "quantity" instead of "current_quantity".
-- Same class of bug as migrations/0008's `COALESCE(pt.target_percent, 0.0)`
-- fix — this migration was the one COALESCE literal that got missed then.
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
