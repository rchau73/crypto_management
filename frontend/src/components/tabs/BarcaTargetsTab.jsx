import { useEffect, useState } from "react";
import {
  Box,
  Button,
  FormControl,
  InputLabel,
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
import { useBarcaTargets } from "../../hooks/useBarcaTargets";
import { zebraRowSx } from "../../styles/tableStyles";
import { getTotalTargetPercent } from "../../utils/allocationMath";
import { STATUS_COLORS } from "../../theme";

const MARKETS = ["BullMarket", "BearMarket"];
const SUM_TOLERANCE = 0.01;

// Manager+/Admin only: editable table + single Save button for BARCA
// targets, scoped to one market profile at a time. A barca with no crypto
// Bull/Bear cycle (e.g. "IBOVE") just gets added to both profiles.
export function BarcaTargetsTab() {
  const [market, setMarket] = useState("BullMarket");
  const { targets, loading, error, save } = useBarcaTargets(market);
  const [rows, setRows] = useState([]);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");
  const [saveSuccess, setSaveSuccess] = useState(false);

  useEffect(() => {
    setRows(targets.map((t) => ({ barca: t.barca, target_percent: t.target_percent })));
    setSaveSuccess(false);
  }, [targets]);

  const sum = getTotalTargetPercent(rows);
  const hasBlankName = rows.some((r) => r.barca.trim() === "");
  const isValid = rows.length > 0 && !hasBlankName && Math.abs(sum - 100) <= SUM_TOLERANCE;

  const updateRow = (index, field, value) => {
    setRows((prev) => prev.map((r, i) => (i === index ? { ...r, [field]: value } : r)));
    setSaveSuccess(false);
  };

  const addRow = () => setRows((prev) => [...prev, { barca: "", target_percent: 0 }]);
  const removeRow = (index) => setRows((prev) => prev.filter((_, i) => i !== index));

  const handleSave = async () => {
    if (!isValid) return;
    setSaving(true);
    setSaveError("");
    try {
      await save(rows.map((r) => ({ barca: r.barca.trim(), target_percent: Number(r.target_percent) || 0 })));
      setSaveSuccess(true);
    } catch (err) {
      setSaveError(err.message);
    }
    setSaving(false);
  };

  return (
    <Box>
      <Box sx={{ display: "flex", alignItems: "center", gap: 2, mt: 2, mb: 1, flexWrap: "wrap" }}>
        <Typography variant="h6">BARCA Targets</Typography>
        <FormControl size="small" sx={{ minWidth: 160 }}>
          <InputLabel>Market</InputLabel>
          <Select label="Market" value={market} onChange={(e) => setMarket(e.target.value)}>
            {MARKETS.map((m) => (
              <MenuItem key={m} value={m}>
                {m}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
      </Box>

      <Typography variant="body2" sx={{ color: "text.secondary", mb: 2, maxWidth: 640 }}>
        Each BARCA bucket (e.g. "Base", "Altcoins", or a new one like "IBOVE") is its own row and only
        applies to the market selected above — add the same bucket to both Bull and Bear if it should
        always be active.
      </Typography>

      {error && (
        <Typography variant="body2" sx={{ color: "error.main", mb: 2 }}>
          {error}
        </Typography>
      )}

      <TableContainer component={Paper} sx={{ maxWidth: 560, mb: 2, overflowX: "auto" }}>
        <Table>
          <TableHead>
            <TableRow>
              <TableCell>BARCA</TableCell>
              <TableCell align="right">Target %</TableCell>
              <TableCell align="right">Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((r, i) => (
              <TableRow key={i} hover sx={zebraRowSx}>
                <TableCell>
                  <TextField
                    size="small"
                    value={r.barca}
                    onChange={(e) => updateRow(i, "barca", e.target.value)}
                    error={r.barca.trim() === ""}
                  />
                </TableCell>
                <TableCell align="right">
                  <TextField
                    size="small"
                    type="number"
                    value={r.target_percent}
                    onChange={(e) => updateRow(i, "target_percent", e.target.value)}
                    sx={{ width: 100 }}
                    slotProps={{ htmlInput: { step: 0.1 } }}
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
                <TableCell colSpan={3} align="center" sx={{ color: "text.secondary" }}>
                  No BARCA targets for {market} yet.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </TableContainer>

      <Box sx={{ display: "flex", alignItems: "center", gap: 2, flexWrap: "wrap" }}>
        <Button size="small" variant="outlined" onClick={addRow}>
          + Add BARCA
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
