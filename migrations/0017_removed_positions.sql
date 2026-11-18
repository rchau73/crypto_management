-- Positions the manager removed from Portfolio Targets (a soft delete).
--
-- The ledger stays append-only: removing a position only adds a row here.
-- A removed position is hidden while its quantity is 0; any ledger write
-- that gives it a quantity again deletes its row here, in the same
-- transaction, so a removal can never hide money.
CREATE TABLE removed_positions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  symbol TEXT NOT NULL,
  -- '' instead of NULL, like portfolio_targets: SQLite's UNIQUE treats
  -- every NULL as distinct.
  group_name TEXT NOT NULL DEFAULT '',
  barca TEXT NOT NULL DEFAULT '',
  asset_class TEXT NOT NULL,
  removed_by INTEGER REFERENCES users(id),
  removed_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(portfolio_id, symbol, group_name, barca, asset_class)
);
