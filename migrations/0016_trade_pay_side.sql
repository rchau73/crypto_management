-- 0016_trade_pay_side.sql
--
-- A trade can now be paid with (or sell into) any holding, not only cash in
-- Caixa: e.g. buying SOL on a SOL/BTC pair, paid with BTC. The log keeps
-- which position paid (group and BARCA, as BTC can sit in several) and the
-- price as the exchange showed it (e.g. 0.00105 BTC per SOL). `price` stays
-- the USD price, worked out from the pair price and the pay asset's live
-- price when the trade was recorded.

ALTER TABLE trades ADD COLUMN cash_group_name TEXT;
ALTER TABLE trades ADD COLUMN cash_barca TEXT;
-- 'USD' or the pay asset's symbol; NULL for trades recorded before 0016.
ALTER TABLE trades ADD COLUMN price_currency TEXT;
ALTER TABLE trades ADD COLUMN price_in_currency REAL CHECK (price_in_currency > 0);
