-- 0009_fix_portfolio_targets_backfill.sql
--
-- 0008's backfill used MAX(target_percent) across a symbol's entire ledger
-- history. That's the same mistake migrations/0007 made and reverted from:
-- for a symbol whose target was *lowered* at some point (e.g. 30% -> 29%),
-- an old, superseded ledger row from before the change can still hold a
-- higher stale value, and MAX() picks that stale value back up — observed
-- live immediately after 0008 ran, reintroducing the exact "target edits
-- can't decrease" bug portfolio_targets was built to fix.
--
-- Fix: re-derive from the single most-recently-created ledger row per key,
-- matching the "latest edit always wins" semantics portfolio_targets is
-- supposed to have from here on.
DELETE FROM portfolio_targets;

INSERT INTO portfolio_targets (symbol, group_name, barca, asset_class, target_percent)
SELECT symbol, group_name, barca, asset_class, COALESCE(target_percent, 0.0)
FROM (
    SELECT
        symbol,
        COALESCE(group_name, '') AS group_name,
        COALESCE(barca, '') AS barca,
        asset_class,
        target_percent,
        ROW_NUMBER() OVER (
            PARTITION BY symbol, COALESCE(group_name, ''), COALESCE(barca, ''), asset_class
            ORDER BY created_at DESC, id DESC
        ) AS rn
    FROM wallet_allocations
)
WHERE rn = 1;
