-- 0014_trades.sql
--
-- Trades recorded in the app ("Record trade" on the Rebalance page), so a
-- buy or sell no longer means editing the wallet CSV and importing it
-- again. The quantity change itself is a normal ledger row in
-- wallet_allocations (written in the same transaction); this table keeps
-- what happened: side, quantity, fill price, exchange and the cash moved.

CREATE TABLE trades (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  side TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
  symbol TEXT NOT NULL,
  group_name TEXT,
  barca TEXT,
  asset_class TEXT NOT NULL,
  -- The exchange/wallet: the ledger's `notes` of the source traded.
  source TEXT,
  quantity REAL NOT NULL CHECK (quantity > 0),
  price REAL NOT NULL CHECK (price > 0),
  cash_symbol TEXT,
  cash_source TEXT,
  cash_quantity REAL,
  recorded_by INTEGER REFERENCES users(id),
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_trades_portfolio ON trades(portfolio_id, created_at);
