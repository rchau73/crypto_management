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
import {
  NEW_SOURCE,
  assetSources,
  buildTradeRequest,
  cashSources,
  defaultCashSource,
  formatQuantity,
  inputNumber,
  parsePositive,
  previewTrade,
  priceOf,
  sourceLabel,
  sourceName,
  tradablePositions,
} from "../utils/trade";

// "Record trade": a buy or sell already made on an exchange, written into
// this account's wallet (no CSV round trip). `preset` comes from a round
// step ({ side, symbol, barca, amount, stepId }) or is null for "any
// position". Mount it only while open, so every opening starts fresh.
export function RecordTradeDialog({ preset, allocations, accountName, onClose, onRecorded }) {
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
  const [priceText, setPriceText] = useState(() => inputNumber(livePrice));
  const [useCash, setUseCash] = useState(true);
  const [cashId, setCashId] = useState(null);
  // null = worked out from the total, until the user types their own.
  const [cashText, setCashText] = useState(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");

  const options = useMemo(() => assetSources(sources || [], symbol, barca), [sources, symbol, barca]);
  const cashOptions = useMemo(() => cashSources(sources || []), [sources]);
  const chosenId = exchangeId ?? options[0]?.id ?? NEW_SOURCE;
  const isNew = chosenId === NEW_SOURCE;
  // A typed name that is an existing exchange ("binance" for "Binance") is
  // that exchange, not a second one (the server matches it the same way).
  const typedName = newExchange.trim().toLowerCase();
  const typedMatch = isNew && typedName ? options.find((o) => sourceName(o.notes).toLowerCase() === typedName) : null;
  const chosen = typedMatch ?? options.find((o) => o.id === chosenId) ?? null;
  const exchange = isNew && !typedMatch ? newExchange : chosen?.notes;
  const held = chosen?.current_quantity ?? 0;

  const cash = cashOptions.find((c) => c.id === cashId) ?? defaultCashSource(cashOptions, exchange);
  const cashOn = useCash && cash !== null;
  const quantity = parsePositive(quantityText);
  const price = parsePositive(priceText);
  const cashPrice = cash ? priceOf(allocations, cash.symbol) : null;
  const autoCash = quantity !== null && price !== null && cashPrice ? inputNumber((quantity * price) / cashPrice) : "";
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
  });
  const problems = [...preview.problems];
  if (isNew && newExchange.trim() === "") problems.push("Name the new exchange.");
  // Every position has at least one source; none means the list is stale.
  const template = options[0] ?? null;
  const ready = sources !== null && template !== null && problems.length === 0 && !saving;
  const verb = side === "buy" ? "buy" : "sell";
  const where = sourceLabel(exchange);

  const changePosition = (value) => {
    const [nextSymbol] = value.split("|");
    setPosition(value);
    setExchangeId(null);
    setQuantityText("");
    setPriceText(inputNumber(priceOf(allocations, nextSymbol)));
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
          price,
          cash: cashOn ? cash : null,
          cashQuantity,
        })
      );
      const cashPart = cashOn ? `, ${side === "buy" ? "paid with" : "into"} ${formatQuantity(cashQuantity)} ${cash.symbol}` : "";
      onRecorded({
        stepId: preset?.stepId ?? null,
        message: `Recorded in ${accountName}: ${side === "buy" ? "bought" : "sold"} ${formatQuantity(quantity)} ${symbol} on ${where} at $${formatNumber(price)}${cashPart}.`,
      });
    } catch (err) {
      setSaveError(err.message);
      setSaving(false);
    }
  };

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

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField
                label={`Quantity (${symbol})`}
                value={quantityText}
                onChange={(e) => {
                  setQuantityText(e.target.value);
                  setCashText(null);
                }}
                slotProps={{ htmlInput: { inputMode: "decimal" } }}
                helperText={preset ? `Pre-filled from the step: $${formatNumber(preset.amount)}` : " "}
                fullWidth
              />
              <TextField
                label="Price (USD)"
                value={priceText}
                onChange={(e) => {
                  setPriceText(e.target.value);
                  setCashText(null);
                }}
                slotProps={{ htmlInput: { inputMode: "decimal" } }}
                helperText={livePrice ? `Live price $${formatNumber(livePrice)}; use your fill price` : "No live price; enter your fill price"}
                fullWidth
              />
            </Stack>
            <Typography sx={{ fontVariantNumeric: "tabular-nums" }}>
              Total: <strong>{preview.total === null ? "—" : `$${formatNumber(preview.total)}`}</strong>
            </Typography>

            <Box sx={{ borderTop: 1, borderColor: "divider", pt: 1 }}>
              <FormControlLabel
                control={<Checkbox checked={cashOn} disabled={cashOptions.length === 0} onChange={(e) => setUseCash(e.target.checked)} />}
                label={
                  cashOptions.length === 0
                    ? "Also move the cash (this account has no Caixa position)"
                    : side === "buy"
                      ? "Also take the cash from Caixa"
                      : "Also put the proceeds in Caixa"
                }
              />
              {cashOn && (
                <Stack direction={{ xs: "column", sm: "row" }} spacing={2} sx={{ mt: 1 }}>
                  <TextField
                    select
                    label="Cash"
                    value={cash.id}
                    onChange={(e) => setCashId(e.target.value)}
                    fullWidth
                  >
                    {cashOptions.map((c) => (
                      <MenuItem key={c.id} value={c.id}>
                        {c.symbol} · {sourceLabel(c.notes)} — {formatQuantity(c.current_quantity)}
                      </MenuItem>
                    ))}
                  </TextField>
                  <TextField
                    label={`${cash.symbol} ${side === "buy" ? "spent" : "received"}`}
                    value={cashValue}
                    onChange={(e) => setCashText(e.target.value)}
                    slotProps={{ htmlInput: { inputMode: "decimal" } }}
                    helperText={cashText === null ? "Worked out from the total; edit to include fees" : " "}
                    fullWidth
                  />
                </Stack>
              )}
            </Box>

            <Box sx={{ bgcolor: "action.hover", borderRadius: 1, px: 1.5, py: 1, fontVariantNumeric: "tabular-nums" }} data-testid="trade-preview">
              <Typography variant="body2" sx={{ fontWeight: 700, mb: 0.5 }}>
                What changes
              </Typography>
              <Typography variant="body2">
                {symbol} on {where}: {formatQuantity(held)} → {preview.newHeld === null ? "—" : formatQuantity(preview.newHeld)}
              </Typography>
              {cashOn && (
                <Typography variant="body2">
                  {cash.symbol} on {sourceLabel(cash.notes)}: {formatQuantity(cash.current_quantity)} →{" "}
                  {preview.newCash === null ? "—" : formatQuantity(preview.newCash)}
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
