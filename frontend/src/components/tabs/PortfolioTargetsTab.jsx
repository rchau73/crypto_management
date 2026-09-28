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
import { fetchBarcaTargets } from "../../api/client";
import { zebraRowSx } from "../../styles/tableStyles";
import { getTotalTargetPercent } from "../../utils/allocationMath";
import { STATUS_COLORS } from "../../theme";

const ASSET_CLASSES = ["crypto", "br-equities", "us-indices"];
const SUM_TOLERANCE = 0.01;

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
export function PortfolioTargetsTab({ active }) {
  const { rows: currentRows, loading, error, save } = usePortfolioTargets(active);
  const [rows, setRows] = useState([]);
  const [barcaOptions, setBarcaOptions] = useState([]);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");
  const [saveSuccess, setSaveSuccess] = useState(false);

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
        new asset (added below) can have a starting quantity. Removing a row does not delete its history;
        set its target to 0 instead.
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
                  <TextField
                    size="small"
                    type="number"
                    value={r.current_quantity}
                    onChange={(e) => updateRow(i, "current_quantity", e.target.value)}
                    disabled={!r.isNew}
                    title={!r.isNew ? "Managed by CSV import / Update Prices, not editable here" : undefined}
                    sx={{ width: 110 }}
                    slotProps={{ htmlInput: { step: "any" } }}
                  />
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
              </TableRow>
            ))}
            {!loading && rows.length === 0 && (
              <TableRow>
                <TableCell colSpan={7} align="center" sx={{ color: "text.secondary" }}>
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
