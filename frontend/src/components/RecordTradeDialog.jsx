import { useMemo, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  ListSubheader,
  MenuItem,
  Stack,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { recordTrade } from "../api/client";
import { usePositionSources } from "../hooks/usePositionSources";
import { useCurrentPortfolio } from "../hooks/useCurrentPortfolio";
import { formatNumber } from "../utils/formatters";
import { barcaShift } from "../utils/move";
import {
  NEW_SOURCE,
  assetSources,
  autoPayQuantity,
  buildTradeRequest,
  defaultCashSource,
  formatQuantity,
  inputNumber,
  livePriceIn,
  parsePositive,
  payOptions,
  previewTrade,
  priceOf,
  priceUnits,
  sourceLabel,
  sourceName,
  tradablePositions,
  usdPrice,
} from "../utils/trade";

function payLabel(c) {
  return `${c.symbol} · ${c.barca} · ${sourceLabel(c.notes)} — ${formatQuantity(c.current_quantity)}`;
}

// "Record trade": a buy or sell already made on an exchange, written into
// this account's wallet (no CSV round trip). `preset` comes from a round
// step ({ side, symbol, barca, amount, stepId }) or is null for "any
// position". Mount it only while open, so every opening starts fresh.
//
// The other side of the trade can be cash in Caixa or any other holding
// (BTC on a SOL/BTC pair). The price is typed in the pair's quote (BTC, or
// USDT) or in USD; the server always gets the USD price, plus the price as
// typed. `barcaActual` (the report's BARCA values) feeds the BARCA shift.
export function RecordTradeDialog({ preset, allocations, barcaActual = [], accountName, onClose, onRecorded }) {
  const { id: portfolioId } = useCurrentPortfolio();
  const { sources, error: loadError } = usePositionSources();
  const positions = useMemo(() => tradablePositions(allocations), [allocations]);

  const [position, setPosition] = useState(() =>
    preset ? `${preset.symbol}|${preset.barca}` : positions[0] ? `${positions[0].symbol}|${positions[0].barca}` : ""
  );
  const [symbol, barca] = position.split("|");
  const livePrice = priceOf(allocations, symbol);

  const [side, setSide] = useState(preset?.side ?? "buy");
  // null = the default pick (the biggest source), once the sources arrive.
  const [exchangeId, setExchangeId] = useState(null);
  const [newExchange, setNewExchange] = useState("");
  const [quantityText, setQuantityText] = useState(() =>
    preset && livePrice ? inputNumber(preset.amount / livePrice) : ""
  );
  // null = on when there is Caixa cash to default to.
  const [useCash, setUseCash] = useState(null);
  const [cashId, setCashId] = useState(null);
  // null = the pay asset when it has a live price, else USD.
  const [unitPick, setUnitPick] = useState(null);
  // null = the live price in the current unit, until the user types one.
  const [priceText, setPriceText] = useState(null);
  // null = worked out from quantity × price, until the user types their own.
  const [cashText, setCashText] = useState(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");

  const options = useMemo(() => assetSources(sources || [], symbol, barca), [sources, symbol, barca]);
  const payable = useMemo(() => payOptions(sources || [], symbol, allocations), [sources, symbol, allocations]);
  const chosenId = exchangeId ?? options[0]?.id ?? NEW_SOURCE;
  const isNew = chosenId === NEW_SOURCE;
  // A typed name that is an existing exchange ("binance" for "Binance") is
  // that exchange, not a second one (the server matches it the same way).
  const typedName = newExchange.trim().toLowerCase();
  const typedMatch = isNew && typedName ? options.find((o) => sourceName(o.notes).toLowerCase() === typedName) : null;
  const chosen = typedMatch ?? options.find((o) => o.id === chosenId) ?? null;
  const exchange = isNew && !typedMatch ? newExchange : chosen?.notes;
  const held = chosen?.current_quantity ?? 0;

  const defaultCash = defaultCashSource(payable, exchange);
  const cash = payable.find((c) => c.id === cashId) ?? defaultCash ?? payable[0] ?? null;
  const cashOn = (useCash ?? defaultCash !== null) && cash !== null;
  const payPrice = cashOn ? priceOf(allocations, cash.symbol) : null;
  const units = priceUnits(cashOn ? cash.symbol : null, payPrice);
  const unit = units.includes(unitPick) ? unitPick : units[0];

  const quantity = parsePositive(quantityText);
  const priceValue = priceText ?? inputNumber(livePriceIn(unit, livePrice, payPrice));
  const price = parsePositive(priceValue);
  const priceUsd = usdPrice(price, unit, payPrice);
  const autoCash = cashOn ? inputNumber(autoPayQuantity({ quantity, price, unit, payPrice })) : "";
  const cashValue = cashText ?? autoCash;
  const cashQuantity = parsePositive(cashValue);

  const preview = previewTrade({
    side,
    held,
    quantity,
    price,
    useCash: cashOn,
    cashHeld: cash?.current_quantity ?? 0,
    cashQuantity,
    cashSymbol: cash?.symbol,
  });
  const problems = [...preview.problems];
  if (isNew && newExchange.trim() === "") problems.push("Name the new exchange.");
  // Every position has at least one source; none means the list is stale.
  const template = options[0] ?? null;
  const ready = sources !== null && template !== null && priceUsd !== null && problems.length === 0 && !saving;
  const verb = side === "buy" ? "buy" : "sell";
  const where = sourceLabel(exchange);
  const priceShown = (p) => (unit === "USD" ? `$${formatNumber(p)}` : `${formatQuantity(p)} ${unit}`);

  // Paying with (or selling into) a holding in another BARCA moves value
  // between BARCAs: show the shares before and after.
  const assetValue = quantity !== null && priceUsd !== null ? quantity * priceUsd : null;
  const payValue = cashOn && cashQuantity !== null && payPrice ? cashQuantity * payPrice : null;
  const shift =
    assetValue !== null && payValue !== null && cash.barca !== barca
      ? barcaShift(
          { barcaActualAllocations: barcaActual },
          side === "buy"
            ? { fromBarca: cash.barca, toBarca: barca, sentValue: payValue, receivedValue: assetValue }
            : { fromBarca: barca, toBarca: cash.barca, sentValue: assetValue, receivedValue: payValue }
        )
      : [];

  const changePosition = (value) => {
    setPosition(value);
    setExchangeId(null);
    setQuantityText("");
    setCashId(null);
    setUnitPick(null);
    setPriceText(null);
    setCashText(null);
  };
  // A new pay asset changes the price unit, so the price starts over.
  const changeCash = (id) => {
    setCashId(id);
    setUnitPick(null);
    setPriceText(null);
    setCashText(null);
  };
  const changeUnit = (value) => {
    setUnitPick(value);
    setPriceText(null);
    setCashText(null);
  };

  const submit = async () => {
    setSaving(true);
    setSaveError("");
    try {
      await recordTrade(
        portfolioId,
        buildTradeRequest({
          side,
          asset: chosen ?? template,
          exchange,
          quantity,
          price: priceUsd,
          cash: cashOn ? cash : null,
          cashQuantity,
          quote: unit === "USD" ? null : { currency: unit, price },
        })
      );
      const at = unit === "USD" ? `$${formatNumber(priceUsd)}` : `${formatQuantity(price)} ${unit} (≈ $${formatNumber(priceUsd)})`;
      const cashPart = cashOn ? `, ${side === "buy" ? "paid with" : "into"} ${formatQuantity(cashQuantity)} ${cash.symbol}` : "";
      onRecorded({
        stepId: preset?.stepId ?? null,
        message: `Recorded in ${accountName}: ${side === "buy" ? "bought" : "sold"} ${formatQuantity(quantity)} ${symbol} on ${where} at ${at}${cashPart}.`,
      });
    } catch (err) {
      setSaveError(err.message);
      setSaving(false);
    }
  };

  const cashItems = payable.filter((c) => c.isCash);
  const holdingItems = payable.filter((c) => !c.isCash);

  return (
    <Dialog open onClose={saving ? undefined : onClose} fullWidth maxWidth="sm" aria-labelledby="record-trade-title">
      <DialogTitle id="record-trade-title" sx={{ fontWeight: 700 }}>
        Record trade{" "}
        <Typography component="span" sx={{ color: "text.secondary" }}>
          in {accountName}
        </Typography>
      </DialogTitle>
      <DialogContent>
        <Typography variant="body2" sx={{ color: "text.secondary", mb: 2 }}>
          A trade you already made on an exchange. It updates this account's quantities right away, with no CSV import.
        </Typography>
        {preset && (
          <Typography sx={{ fontWeight: 700, mb: 2 }}>
            {symbol} · {barca}
          </Typography>
        )}

        {sources === null ? (
          <Box sx={{ py: 4, textAlign: "center" }}>
            <CircularProgress size={28} />
          </Box>
        ) : (
          <Stack spacing={2}>
            {loadError && <Alert severity="error">Couldn't load the exchanges: {loadError}</Alert>}
            {!preset && (
              <TextField select label="Asset" value={position} onChange={(e) => changePosition(e.target.value)}>
                {positions.map((p) => (
                  <MenuItem key={`${p.symbol}|${p.barca}`} value={`${p.symbol}|${p.barca}`}>
                    {p.symbol} · {p.barca}
                  </MenuItem>
                ))}
              </TextField>
            )}

            <ToggleButtonGroup exclusive value={side} onChange={(_, next) => next && setSide(next)} aria-label="Side">
              <ToggleButton value="buy" sx={{ px: 3, minHeight: 44 }}>
                Buy
              </ToggleButton>
              <ToggleButton value="sell" sx={{ px: 3, minHeight: 44 }}>
                Sell
              </ToggleButton>
            </ToggleButtonGroup>

            <TextField
              select
              label="Exchange"
              value={chosenId}
              onChange={(e) => setExchangeId(e.target.value)}
              helperText={isNew ? "A new exchange for this asset starts at 0." : `Holds ${formatQuantity(held)} ${symbol}`}
            >
              {options.map((o) => (
                <MenuItem key={o.id} value={o.id}>
                  {sourceLabel(o.notes)} — {formatQuantity(o.current_quantity)} {symbol}
                  {options.some((x) => x.group_name !== o.group_name) ? ` (${o.group_name})` : ""}
                </MenuItem>
              ))}
              <MenuItem value={NEW_SOURCE}>Another exchange…</MenuItem>
            </TextField>
            {isNew && (
              <TextField
                label="Exchange name"
                value={newExchange}
                onChange={(e) => setNewExchange(e.target.value)}
                helperText={
                  typedMatch
                    ? `That's ${sourceLabel(typedMatch.notes)}, which holds ${formatQuantity(held)} ${symbol}: the trade goes there`
                    : " "
                }
                autoFocus
              />
            )}

            <TextField
              label={`Quantity (${symbol})`}
              value={quantityText}
              onChange={(e) => {
                setQuantityText(e.target.value);
                setCashText(null);
              }}
              slotProps={{ htmlInput: { inputMode: "decimal" } }}
              helperText={preset ? `Pre-filled from the step: $${formatNumber(preset.amount)}` : " "}
            />

            <Box sx={{ borderTop: 1, borderColor: "divider", pt: 1 }}>
              <FormControlLabel
                control={
                  <Checkbox
                    checked={cashOn}
                    disabled={payable.length === 0}
                    onChange={(e) => setUseCash(e.target.checked)}
                  />
                }
                label={
                  payable.length === 0
                    ? "Also move what paid (this account holds nothing else)"
                    : side === "buy"
                      ? "Also take what paid from my holdings"
                      : "Also put the proceeds in my holdings"
                }
              />
              {cashOn && (
                <TextField
                  select
                  label={side === "buy" ? "Paid with" : "Proceeds to"}
                  value={cash.id}
                  onChange={(e) => changeCash(e.target.value)}
                  helperText={cash.isCash ? "Cash in Caixa" : `A ${cash.symbol} holding in ${cash.barca}: its BARCA share changes too`}
                  fullWidth
                  sx={{ mt: 1 }}
                >
                  {cashItems.length > 0 && <ListSubheader>Cash (Caixa)</ListSubheader>}
                  {cashItems.map((c) => (
                    <MenuItem key={c.id} value={c.id}>
                      {payLabel(c)}
                    </MenuItem>
                  ))}
                  {holdingItems.length > 0 && <ListSubheader>Other holdings</ListSubheader>}
                  {holdingItems.map((c) => (
                    <MenuItem key={c.id} value={c.id}>
                      {payLabel(c)}
                    </MenuItem>
                  ))}
                </TextField>
              )}
            </Box>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField
                label={`Price (${unit} per ${symbol})`}
                value={priceValue}
                onChange={(e) => {
                  setPriceText(e.target.value);
                  setCashText(null);
                }}
                slotProps={{ htmlInput: { inputMode: "decimal" } }}
                helperText={
                  unit !== "USD"
                    ? priceUsd !== null
                      ? `≈ $${formatNumber(priceUsd)} per ${symbol} at the live ${unit} price; use your fill price`
                      : `Your fill price in ${unit}`
                    : livePrice
                      ? `Live price $${formatNumber(livePrice)}; use your fill price`
                      : "No live price; enter your fill price"
                }
                fullWidth
              />
              {units.length > 1 && (
                <TextField
                  select
                  label="Price in"
                  value={unit}
                  onChange={(e) => changeUnit(e.target.value)}
                  sx={{ minWidth: 120 }}
                >
                  {units.map((u) => (
                    <MenuItem key={u} value={u}>
                      {u}
                    </MenuItem>
                  ))}
                </TextField>
              )}
            </Stack>
            {cashOn && (
              <TextField
                label={`${cash.symbol} ${side === "buy" ? "spent" : "received"}`}
                value={cashValue}
                onChange={(e) => setCashText(e.target.value)}
                slotProps={{ htmlInput: { inputMode: "decimal" } }}
                helperText={cashText === null ? "Worked out from quantity × price; edit to include fees" : " "}
              />
            )}
            <Typography sx={{ fontVariantNumeric: "tabular-nums" }}>
              Total:{" "}
              <strong>
                {preview.total === null ? "—" : priceShown(preview.total)}
                {unit !== "USD" && assetValue !== null ? ` (≈ $${formatNumber(assetValue)})` : ""}
              </strong>
            </Typography>

            <Box sx={{ bgcolor: "action.hover", borderRadius: 1, px: 1.5, py: 1, fontVariantNumeric: "tabular-nums" }} data-testid="trade-preview">
              <Typography variant="body2" sx={{ fontWeight: 700, mb: 0.5 }}>
                What changes
              </Typography>
              <Typography variant="body2">
                {symbol} on {where}: {formatQuantity(held)} → {preview.newHeld === null ? "—" : formatQuantity(preview.newHeld)}
              </Typography>
              {cashOn && (
                <Typography variant="body2">
                  {cash.symbol}
                  {cash.isCash ? "" : ` · ${cash.barca}`} on {sourceLabel(cash.notes)}: {formatQuantity(cash.current_quantity)} →{" "}
                  {preview.newCash === null ? "—" : formatQuantity(preview.newCash)}
                </Typography>
              )}
              {shift.length > 0 && (
                <Typography variant="body2" sx={{ color: "text.secondary", mt: 0.5 }}>
                  {shift.map((s) => `${s.barca} ${formatNumber(s.before)}% → ${formatNumber(s.after)}%`).join(" · ")}
                </Typography>
              )}
              {problems.map((p) => (
                <Typography key={p} variant="body2" sx={{ color: "warning.light" }}>
                  {p}
                </Typography>
              ))}
            </Box>
            {saveError && <Alert severity="error">{saveError}</Alert>}
          </Stack>
        )}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="contained" onClick={submit} disabled={!ready}>
          {saving ? "Recording…" : `Record ${verb} in ${accountName}`}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
