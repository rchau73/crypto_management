import { useEffect, useMemo, useState } from "react";
import {
  Autocomplete,
  Box,
  Button,
  FormControl,
  MenuItem,
  Paper,
  Select,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from "@mui/material";
import { usePortfolioTargets } from "../../hooks/usePortfolioTargets";
import { correctWalletQuantity, fetchBarcaTargets } from "../../api/client";
import { zebraRowSx } from "../../styles/tableStyles";
import { getTotalTargetPercent } from "../../utils/allocationMath";
import { STATUS_COLORS } from "../../theme";

const ASSET_CLASSES = ["crypto", "br-equities", "us-indices"];
const SUM_TOLERANCE = 0.01;

// `wallet_allocations_current` joins every distinct `notes` value for a
// (symbol, group, barca, asset_class) key with " | " (see migrations/0008's
// view definition). A single-notes (or notes-less) row maps 1:1 to one
// ledger source, so correcting it is unambiguous; a row with " | " in its
// notes is an aggregate of several sources and isn't safe to correct from
// here (see PortfolioTargetsTab's doc comment).
function isSingleSource(row) {
  return !row.notes || !row.notes.includes(" | ");
}

function blankRow() {
  return {
    symbol: "",
    group_name: "",
    barca: "",
    asset_class: "crypto",
    target_percent: 0,
    current_quantity: 0,
    last_price: null,
    notes: "",
    created_at: null,
    isNew: true,
  };
}

// Manager+/Admin only: editable table + single Save button for portfolio
// (wallet_allocations) targets, across the whole portfolio (all asset
// classes together — see migrations/0006 for why this isn't scoped
// per-asset-class). "Barca" suggests names already defined on the BARCA
// tab, but accepts a new one too.
//
// Quantity/notes are read-only for existing rows and NOT editable here —
// `current_quantity` here can be the SUM of several distinct ledger entries
// (e.g. holdings across multiple wallets) collapsed into one displayed row,
// and `notes` can be their concatenated labels. Saving must never round-trip
// that aggregate back as a single new entry: because it's a fresh, distinct
// "notes" value, the aggregation view would treat it as one more source and
// ADD its quantity on top of the originals instead of replacing them,
// silently doubling holdings. Only a genuinely new row (added below) has no
// prior entries to conflict with, so its quantity/notes are safe to set.
//
// A single-source existing row (see `isSingleSource`) gets a "Corrigir"
// control instead: it calls a separate, narrow endpoint
// (`correctWalletQuantity`) that appends one new ledger row for that exact
// source, immediately — not part of the Save All batch above. A
// multi-source row doesn't get this control, since there'd be no way to
// tell which of its concatenated sources to fix.
export function PortfolioTargetsTab({ active }) {
  const { rows: currentRows, loading, error, save } = usePortfolioTargets(active);
  const [rows, setRows] = useState([]);
  const [barcaOptions, setBarcaOptions] = useState([]);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [correctingRow, setCorrectingRow] = useState(null);
  const [correctionValue, setCorrectionValue] = useState("");
  const [correctionSaving, setCorrectionSaving] = useState(false);
  const [correctionError, setCorrectionError] = useState("");

  useEffect(() => {
    setRows(currentRows.map((r) => ({ ...r, isNew: false })));
    setSaveSuccess(false);
  }, [currentRows]);

  useEffect(() => {
    if (!active) return;
    Promise.all([fetchBarcaTargets("BullMarket"), fetchBarcaTargets("BearMarket")])
      .then(([bull, bear]) => {
        const names = new Set([...bull, ...bear].map((t) => t.barca));
        setBarcaOptions(Array.from(names).sort());
      })
      .catch(() => setBarcaOptions([]));
  }, [active]);

  // Suggestions to avoid typos on an existing Group/BARCA — sourced from
  // whatever's already in use across the table, plus (for BARCA) the
  // dedicated BARCA Targets table fetched above. Free typing a new value is
  // still allowed (Autocomplete freeSolo), since a manager can legitimately
  // introduce a new group or barca from here.
  const groupOptions = useMemo(
    () => Array.from(new Set(rows.map((r) => r.group_name).filter(Boolean))).sort(),
    [rows]
  );
  const combinedBarcaOptions = useMemo(() => {
    const fromRows = rows.map((r) => r.barca).filter(Boolean);
    return Array.from(new Set([...barcaOptions, ...fromRows])).sort();
  }, [barcaOptions, rows]);

  const sum = getTotalTargetPercent(rows);
  const hasBlankSymbol = rows.some((r) => r.symbol.trim() === "");
  const isValid = rows.length > 0 && !hasBlankSymbol && Math.abs(sum - 100) <= SUM_TOLERANCE;

  const updateRow = (index, field, value) => {
    setRows((prev) => prev.map((r, i) => (i === index ? { ...r, [field]: value } : r)));
    setSaveSuccess(false);
  };

  const addRow = () => setRows((prev) => [...prev, blankRow()]);
  const removeRow = (index) => {
    setRows((prev) => prev.filter((_, i) => i !== index));
    setSaveSuccess(false);
  };

  const startCorrection = (index) => {
    setCorrectingRow(index);
    setCorrectionValue(String(rows[index].current_quantity ?? 0));
    setCorrectionError("");
  };
  const cancelCorrection = () => {
    setCorrectingRow(null);
    setCorrectionError("");
  };
  const submitCorrection = async (index) => {
    const r = rows[index];
    const newQuantity = Number(correctionValue) || 0;
    setCorrectionSaving(true);
    setCorrectionError("");
    try {
      await correctWalletQuantity({
        symbol: r.symbol,
        group_name: r.group_name || null,
        barca: r.barca || null,
        asset_class: r.asset_class,
        notes: r.notes || null,
        current_quantity: newQuantity,
      });
      setCorrectingRow(null);
      // Update only this row's quantity in place — refetching the whole
      // table here (via the hook's `refresh`) would overwrite every other
      // row with its last-saved server state, silently discarding any
      // Target %/Group/BARCA edit the user made elsewhere in the table
      // that hasn't been submitted via Save All yet.
      setRows((prev) =>
        prev.map((row, i) => (i === index ? { ...row, current_quantity: newQuantity } : row))
      );
    } catch (err) {
      setCorrectionError(err.message);
    }
    setCorrectionSaving(false);
  };

  const handleSave = async () => {
    if (!isValid) return;
    setSaving(true);
    setSaveError("");
    try {
      await save(
        rows.map((r) => ({
          symbol: r.symbol.trim(),
          group_name: r.group_name?.trim() || null,
          barca: r.barca?.trim() || null,
          asset_class: r.asset_class,
          target_percent: Number(r.target_percent) || 0,
          // Existing rows never resubmit quantity/notes/price — see the
          // component doc comment for why that's unsafe. Only a brand-new
          // row (no prior ledger entries) can safely carry a starting value.
          current_quantity: r.isNew ? Number(r.current_quantity) || 0 : 0,
          notes: r.isNew ? r.notes?.trim() || null : null,
          last_price: null,
        }))
      );
      setSaveSuccess(true);
    } catch (err) {
      setSaveError(err.message);
    }
    setSaving(false);
  };

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 1 }}>
        Portfolio Targets
      </Typography>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: 2, maxWidth: 640 }}>
        Every row's target % here — across the whole portfolio, every asset class together — must sum to
        100%. Quantity and notes are managed by CSV import / "Update Prices" and shown read-only; a brand
        new asset (added below) can have a starting quantity. A single-source row (one wallet/broker) can
        have its quantity fixed via "Corrigir", applied immediately; a row that sums several sources shows
        "multi-fonte" instead, since there'd be no way to tell which source to correct. Remove takes the
        row's target out of the portfolio (its quantity history in the ledger isn't deleted — this only
        affects the target, not past records) and drops it from the 100% sum until Save.
      </Typography>

      {error && (
        <Typography variant="body2" sx={{ color: "error.main", mb: 2 }}>
          {error}
        </Typography>
      )}

      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto", mb: 2 }}>
        <Table sx={{ minWidth: 960 }}>
          <TableHead>
            <TableRow>
              <TableCell>Symbol</TableCell>
              <TableCell>Group</TableCell>
              <TableCell>BARCA</TableCell>
              <TableCell>Asset Class</TableCell>
              <TableCell align="right">Target %</TableCell>
              <TableCell align="right">Quantity</TableCell>
              <TableCell>Notes</TableCell>
              <TableCell align="right">Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((r, i) => (
              <TableRow key={i} hover sx={zebraRowSx}>
                <TableCell>
                  <TextField
                    size="small"
                    value={r.symbol}
                    onChange={(e) => updateRow(i, "symbol", e.target.value)}
                    error={r.symbol.trim() === ""}
                    sx={{ width: 110 }}
                  />
                </TableCell>
                <TableCell>
                  <Autocomplete
                    freeSolo
                    size="small"
                    options={groupOptions}
                    value={r.group_name || ""}
                    onInputChange={(_, value) => updateRow(i, "group_name", value)}
                    renderInput={(params) => <TextField {...params} sx={{ width: 120 }} />}
                  />
                </TableCell>
                <TableCell>
                  <Autocomplete
                    freeSolo
                    size="small"
                    options={combinedBarcaOptions}
                    value={r.barca || ""}
                    onInputChange={(_, value) => updateRow(i, "barca", value)}
                    renderInput={(params) => <TextField {...params} sx={{ width: 140 }} />}
                  />
                </TableCell>
                <TableCell>
                  <FormControl size="small" sx={{ minWidth: 130 }}>
                    <Select value={r.asset_class} onChange={(e) => updateRow(i, "asset_class", e.target.value)}>
                      {ASSET_CLASSES.map((c) => (
                        <MenuItem key={c} value={c}>
                          {c}
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                </TableCell>
                <TableCell align="right">
                  <TextField
                    size="small"
                    type="number"
                    value={r.target_percent}
                    onChange={(e) => updateRow(i, "target_percent", e.target.value)}
                    sx={{ width: 90 }}
                    slotProps={{ htmlInput: { step: 0.1 } }}
                  />
                </TableCell>
                <TableCell align="right">
                  {r.isNew ? (
                    <TextField
                      size="small"
                      type="number"
                      value={r.current_quantity}
                      onChange={(e) => updateRow(i, "current_quantity", e.target.value)}
                      sx={{ width: 110 }}
                      slotProps={{ htmlInput: { step: "any" } }}
                    />
                  ) : correctingRow === i ? (
                    <Box sx={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 0.5 }}>
                      <Box sx={{ display: "flex", gap: 0.5 }}>
                        <TextField
                          size="small"
                          type="number"
                          autoFocus
                          value={correctionValue}
                          onChange={(e) => setCorrectionValue(e.target.value)}
                          sx={{ width: 90 }}
                          slotProps={{ htmlInput: { step: "any" } }}
                        />
                        <Button size="small" disabled={correctionSaving} onClick={() => submitCorrection(i)}>
                          Salvar
                        </Button>
                        <Button size="small" disabled={correctionSaving} onClick={cancelCorrection}>
                          Cancelar
                        </Button>
                      </Box>
                      {correctionError && (
                        <Typography variant="caption" sx={{ color: "error.main" }}>
                          {correctionError}
                        </Typography>
                      )}
                    </Box>
                  ) : (
                    <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, justifyContent: "flex-end" }}>
                      <TextField
                        size="small"
                        type="number"
                        value={r.current_quantity}
                        disabled
                        title="Managed by CSV import / Update Prices — use Corrigir to fix a wrong value"
                        sx={{ width: 90 }}
                        slotProps={{ htmlInput: { step: "any" } }}
                      />
                      {isSingleSource(r) ? (
                        <Button size="small" onClick={() => startCorrection(i)}>
                          Corrigir
                        </Button>
                      ) : (
                        <Typography
                          variant="caption"
                          title="Soma mais de uma fonte (notes concatenados) — correção direta não suportada ainda"
                          sx={{ color: "text.disabled", px: 0.5 }}
                        >
                          multi-fonte
                        </Typography>
                      )}
                    </Box>
                  )}
                </TableCell>
                <TableCell>
                  <TextField
                    size="small"
                    value={r.notes || ""}
                    onChange={(e) => updateRow(i, "notes", e.target.value)}
                    disabled={!r.isNew}
                    title={!r.isNew ? "Managed by CSV import / Update Prices, not editable here" : undefined}
                    sx={{ width: 140 }}
                  />
                </TableCell>
                <TableCell align="right">
                  <Button size="small" color="error" onClick={() => removeRow(i)}>
                    Remove
                  </Button>
                </TableCell>
              </TableRow>
            ))}
            {!loading && rows.length === 0 && (
              <TableRow>
                <TableCell colSpan={8} align="center" sx={{ color: "text.secondary" }}>
                  No portfolio targets yet.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </TableContainer>

      <Box sx={{ display: "flex", alignItems: "center", gap: 2, flexWrap: "wrap" }}>
        <Button size="small" variant="outlined" onClick={addRow}>
          + Add Asset
        </Button>
        <Typography variant="body2" sx={{ color: isValid ? STATUS_COLORS.good : STATUS_COLORS.critical, fontWeight: 600 }}>
          Sum: {sum.toFixed(2)}%
        </Typography>
        <Button variant="contained" disabled={!isValid || saving} onClick={handleSave}>
          Save All
        </Button>
        {saveSuccess && (
          <Typography variant="body2" sx={{ color: "success.main" }}>
            Saved.
          </Typography>
        )}
        {saveError && (
          <Typography variant="body2" sx={{ color: "error.main" }}>
            {saveError}
          </Typography>
        )}
      </Box>
    </Box>
  );
}
