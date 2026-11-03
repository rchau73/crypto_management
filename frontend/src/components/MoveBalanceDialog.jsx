import { useMemo, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  MenuItem,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { recordTransfer } from "../api/client";
import { useCurrentPortfolio } from "../hooks/useCurrentPortfolio";
import { usePositionSources } from "../hooks/usePositionSources";
import { formatNumber } from "../utils/formatters";
import { formatQuantity, inputNumber, parsePositive, priceOf, sourceLabel, sourceName } from "../utils/trade";
import {
  QUICK_PERCENTS,
  barcaNames,
  barcaShift,
  buildTransferRequest,
  defaultGroup,
  destination,
  exchangeNames,
  groupsIn,
  isSamePlace,
  movableSymbols,
  originOptions,
  previewMove,
} from "../utils/move";

function place(barca, group, exchange) {
  return `${barca}${group && group !== barca ? ` · ${group}` : ""} · ${sourceLabel(exchange)}`;
}

// "Move balance": a quantity of one asset moved to another BARCA/group
// and/or exchange (e.g. 15% of BTC from Base to RendaPassiva, or 0.5 BTC
// from Binance to Deribit), with no trade. A new destination position
// starts at a 0% asset target. Mount it only while open.
export function MoveBalanceDialog({ report, accountName, onClose, onMoved }) {
  const { id: portfolioId } = useCurrentPortfolio();
  const { sources, error: loadError } = usePositionSources();
  const all = useMemo(() => sources || [], [sources]);
  const symbols = useMemo(() => movableSymbols(all), [all]);
  const barcas = useMemo(() => barcaNames(report), [report]);

  // null = the default pick, worked out once the sources arrive.
  const [symbolPick, setSymbolPick] = useState(null);
  const [originId, setOriginId] = useState(null);
  const [quantityText, setQuantityText] = useState("");
  // null = same as the quantity sent (no fee).
  const [receivedText, setReceivedText] = useState(null);
  const [barcaPick, setBarcaPick] = useState(null);
  const [groupText, setGroupText] = useState(null);
  const [exchangeText, setExchangeText] = useState(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");

  const symbol = symbolPick ?? symbols[0] ?? "";
  const origins = useMemo(() => originOptions(all, symbol), [all, symbol]);
  const origin = origins.find((o) => o.id === originId) ?? origins[0] ?? null;
  const held = origin?.current_quantity ?? 0;
  const barca = barcaPick ?? origin?.barca ?? "";
  const group = groupText ?? defaultGroup(all, symbol, barca);
  // Same exchange by default: a BARCA move usually stays where it is.
  const exchange = exchangeText ?? sourceName(origin?.notes);

  const quantity = parsePositive(quantityText);
  const received = receivedText === null ? quantity : parsePositive(receivedText);
  const dest = origin
    ? destination(all, { symbol, assetClass: origin.asset_class, group, barca, exchange })
    : { held: 0, newPosition: false };
  const preview = previewMove({
    held,
    quantity,
    received,
    destHeld: dest.held,
    samePlace: origin !== null && isSamePlace(origin, { group, barca, exchange }),
    group,
    barca,
  });
  const price = priceOf(report.allocations || [], symbol);
  const shift =
    origin && price && quantity !== null && received !== null
      ? barcaShift(report, { fromBarca: origin.barca, toBarca: barca, sentValue: quantity * price, receivedValue: received * price })
      : [];
  const ready = sources !== null && origin !== null && preview.problems.length === 0 && !saving;

  const pickSymbol = (value) => {
    setSymbolPick(value);
    setOriginId(null);
    setQuantityText("");
    setReceivedText(null);
    setBarcaPick(null);
    setGroupText(null);
    setExchangeText(null);
  };
  const pickOrigin = (id) => {
    setOriginId(id);
    setQuantityText("");
    setReceivedText(null);
    setExchangeText(null);
  };
  const pickBarca = (value) => {
    setBarcaPick(value);
    setGroupText(null);
  };
  const fill = (percent) => {
    setQuantityText(percent === 100 ? String(held) : inputNumber((held * percent) / 100));
    setReceivedText(null);
  };

  const submit = async () => {
    setSaving(true);
    setSaveError("");
    try {
      const outcome = await recordTransfer(
        portfolioId,
        buildTransferRequest({ origin, group, barca, exchange, quantity, received })
      );
      const fee = preview.fee > 0 ? ` (${formatQuantity(received)} arrived after the fee)` : "";
      const fresh = outcome?.new_position ? ` It's a new position with a 0% target; set it in Targets.` : "";
      onMoved({
        message: `Moved in ${accountName}: ${formatQuantity(quantity)} ${symbol} from ${place(origin.barca, origin.group_name, origin.notes)} to ${place(barca, group.trim(), exchange)}${fee}.${fresh}`,
      });
    } catch (err) {
      setSaveError(err.message);
      setSaving(false);
    }
  };

  return (
    <Dialog open onClose={saving ? undefined : onClose} fullWidth maxWidth="sm" aria-labelledby="move-balance-title">
      <DialogTitle id="move-balance-title" sx={{ fontWeight: 700 }}>
        Move balance{" "}
        <Typography component="span" sx={{ color: "text.secondary" }}>
          in {accountName}
        </Typography>
      </DialogTitle>
      <DialogContent>
        <Typography variant="body2" sx={{ color: "text.secondary", mb: 2 }}>
          Move a quantity to another BARCA, another exchange, or both. Nothing is bought or sold: the value only changes place.
        </Typography>

        {sources === null ? (
          <Box sx={{ py: 4, textAlign: "center" }}>
            <CircularProgress size={28} />
          </Box>
        ) : symbols.length === 0 ? (
          <Alert severity={loadError ? "error" : "info"}>
            {loadError ? `Couldn't load the exchanges: ${loadError}` : "This account holds nothing to move yet."}
          </Alert>
        ) : (
          <Stack spacing={2}>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField select label="Asset" value={symbol} onChange={(e) => pickSymbol(e.target.value)} sx={{ minWidth: 140 }}>
                {symbols.map((s) => (
                  <MenuItem key={s} value={s}>
                    {s}
                  </MenuItem>
                ))}
              </TextField>
              <TextField select label="From" value={origin?.id ?? ""} onChange={(e) => pickOrigin(e.target.value)} fullWidth>
                {origins.map((o) => (
                  <MenuItem key={o.id} value={o.id}>
                    {place(o.barca, o.group_name, o.notes)} — {formatQuantity(o.current_quantity)}
                  </MenuItem>
                ))}
              </TextField>
            </Stack>

            <Box>
              <TextField
                label={`Quantity to move (${symbol})`}
                value={quantityText}
                onChange={(e) => {
                  setQuantityText(e.target.value);
                  setReceivedText(null);
                }}
                slotProps={{ htmlInput: { inputMode: "decimal" } }}
                helperText={`Holds ${formatQuantity(held)} ${symbol}${quantity !== null && held > 0 ? ` · ${formatNumber((quantity / held) * 100)}% of it` : ""}`}
                fullWidth
              />
              <Stack direction="row" useFlexGap spacing={1} sx={{ mt: 1, flexWrap: "wrap" }}>
                {QUICK_PERCENTS.map((p) => (
                  <Chip key={p} label={p === 100 ? "All" : `${p}%`} onClick={() => fill(p)} variant="outlined" sx={{ minHeight: 36 }} />
                ))}
              </Stack>
            </Box>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField select label="To BARCA" value={barca} onChange={(e) => pickBarca(e.target.value)} sx={{ minWidth: 180 }}>
                {barcas.map((b) => (
                  <MenuItem key={b} value={b}>
                    {b}
                  </MenuItem>
                ))}
              </TextField>
              <Autocomplete
                freeSolo
                options={groupsIn(all, barca)}
                inputValue={group}
                onInputChange={(_, value) => setGroupText(value)}
                renderInput={(params) => <TextField {...params} label="Group" />}
                fullWidth
              />
            </Stack>
            <Autocomplete
              freeSolo
              options={exchangeNames(all)}
              inputValue={exchange}
              onInputChange={(_, value) => setExchangeText(value)}
              renderInput={(params) => (
                <TextField {...params} label="To exchange" helperText="Pick one or type a new one, e.g. Deribit Crypto Options" />
              )}
            />
            <TextField
              label={`Quantity received (${symbol})`}
              value={receivedText ?? (quantity === null ? "" : quantityText)}
              onChange={(e) => setReceivedText(e.target.value)}
              slotProps={{ htmlInput: { inputMode: "decimal" } }}
              helperText={preview.fee > 0 ? `Fee: ${formatQuantity(preview.fee)} ${symbol}` : "Lower it if a withdrawal fee was taken"}
            />

            <Box sx={{ bgcolor: "action.hover", borderRadius: 1, px: 1.5, py: 1, fontVariantNumeric: "tabular-nums" }} data-testid="move-preview">
              <Typography variant="body2" sx={{ fontWeight: 700, mb: 0.5 }}>
                What changes
              </Typography>
              {origin && (
                <Typography variant="body2">
                  {symbol} {place(origin.barca, origin.group_name, origin.notes)}: {formatQuantity(held)} →{" "}
                  {preview.fromAfter === null ? "—" : formatQuantity(preview.fromAfter)}
                </Typography>
              )}
              <Typography variant="body2">
                {symbol} {place(barca, group.trim(), exchange)}: {formatQuantity(dest.held)} →{" "}
                {preview.toAfter === null ? "—" : formatQuantity(preview.toAfter)}
                {dest.newPosition ? " (new position, 0% target: set it in Targets)" : ""}
              </Typography>
              {shift.length > 0 && (
                <Typography variant="body2" sx={{ color: "text.secondary", mt: 0.5 }}>
                  {shift.map((s) => `${s.barca} ${formatNumber(s.before)}% → ${formatNumber(s.after)}%`).join(" · ")}
                </Typography>
              )}
              {preview.problems.map((p) => (
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
          {saving ? "Moving…" : `Move ${quantity === null ? "" : `${formatQuantity(quantity)} `}${symbol} in ${accountName}`}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
