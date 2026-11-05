-- 0015_transfers.sql
--
-- Balance moves recorded in the app ("Move balance" on the Rebalance page):
-- a quantity of one asset taken from one source (BARCA/group + exchange)
-- and put in another, e.g. 0.5 BTC from Base on Binance to RendaPassiva on
-- Deribit. No price, no trade: the value only changes place. The two
-- quantity changes are normal ledger rows in wallet_allocations, written in
-- the same transaction; this table keeps what was moved and any fee
-- (quantity_sent - quantity_received, e.g. a network withdrawal fee).

CREATE TABLE transfers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
  symbol TEXT NOT NULL,
  asset_class TEXT NOT NULL,
  from_group_name TEXT,
  from_barca TEXT,
  from_source TEXT,
  to_group_name TEXT,
  to_barca TEXT,
  to_source TEXT,
  quantity_sent REAL NOT NULL CHECK (quantity_sent > 0),
  quantity_received REAL NOT NULL CHECK (quantity_received > 0 AND quantity_received <= quantity_sent),
  recorded_by INTEGER REFERENCES users(id),
  created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_transfers_portfolio ON transfers(portfolio_id, created_at);
