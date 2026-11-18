import { useEffect, useMemo, useRef, useState } from "react";
import {
  Autocomplete,
  Box,
  Button,
  FormControl,
  FormControlLabel,
  ListItemText,
  MenuItem,
  Paper,
  Select,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableRow,
  TextField,
  Typography,
} from "@mui/material";
import { usePortfolioTargets } from "../../hooks/usePortfolioTargets";
import { useCurrentPortfolio } from "../../hooks/useCurrentPortfolio";
import { useSortableData } from "../../hooks/useSortableData";
import { SortableTableHead } from "../SortableTableHead";
import { fetchBarcaTargets } from "../../api/client";
import { zebraRowSx } from "../../styles/tableStyles";
import { STATUS_COLORS } from "../../theme";
import { QuantityCell } from "../QuantityCell";
import { TargetRowActions } from "../TargetRowActions";
import {
  ASSET_CLASSES,
  ASSET_CLASS_HINTS,
  blankRow,
  filterRows,
  isKept,
  sortValue,
  toSaveRequest,
  uniqueSorted,
  validateRows,
} from "../../utils/portfolioTargets";
import { getTotalTargetPercent } from "../../utils/allocationMath";

const COLUMNS = [
  { key: "symbol", label: "Symbol", filterable: true },
  { key: "group_name", label: "Group", filterable: true },
  { key: "barca", label: "BARCA", filterable: true },
  { key: "asset_class", label: "Asset Class", filterable: true },
  { key: "target_percent", label: "Target %", align: "right", filterable: true },
  { key: "current_quantity", label: "Quantity", align: "right", filterable: true },
  { key: "notes", label: "Notes", filterable: true },
  { key: "actions", label: "Actions", align: "right", sortable: false },
];

const NO_FILTERS = Object.fromEntries(COLUMNS.filter((c) => c.filterable).map((c) => [c.key, ""]));

// BARCA names from both market profiles of the current portfolio, used only
// as typing suggestions.
function useBarcaNameSuggestions() {
  const { id: portfolioId } = useCurrentPortfolio();
  const [names, setNames] = useState([]);
  useEffect(() => {
    Promise.all([fetchBarcaTargets(portfolioId, "BullMarket"), fetchBarcaTargets(portfolioId, "BearMarket")])
      .then(([bull, bear]) => setNames(uniqueSorted([...bull, ...bear].map((t) => t.barca))))
      .catch(() => setNames([]));
  }, [portfolioId]);
  return names;
}

// Manager+ tab: edit every position's target % (all asset classes together,
// must sum to 100%) and save them in one go with "Save All".
//
// For an existing row, only the target is editable. Its symbol/group/BARCA/
// asset class identify its holdings in the ledger — renaming would move the
// target away from the holdings — so to "rename", add a new row, set the old
// one's quantity to 0 and remove it. Quantity rules live in QuantityCell,
// remove/restore rules in TargetRowActions; removed rows are hidden unless
// "Show removed" is on.
//
// Every column can be sorted (click its header) and filtered (the box under
// it). Rows are edited by a stable `_id`, never by their position on screen,
// since sorting/filtering changes that. The Sum and "Save All" always cover
// every row, filtered out or not. New rows stay at the bottom, unsorted, so
// they don't jump around while being typed in.
export function PortfolioTargetsTab() {
  const { rows: savedRows, loading, error, save } = usePortfolioTargets();
  const [rows, setRows] = useState([]);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [filters, setFilters] = useState(NO_FILTERS);
  const [showRemoved, setShowRemoved] = useState(false);
  const barcaNames = useBarcaNameSuggestions();
  const nextId = useRef(0);
  const withId = (row) => ({ ...row, _id: nextId.current++ });

  useEffect(() => {
    setRows(savedRows.map((r) => withId({ ...r, isNew: false })));
    setSaveSuccess(false);
  }, [savedRows]);

  const savedVisible = useMemo(
    () => filterRows(rows.filter((r) => !r.isNew && (showRemoved || !r.removed)), filters),
    [rows, filters, showRemoved]
  );
  const removedCount = rows.filter((r) => r.removed).length;
  const { sortConfig, toggleSort, sortedRows } = useSortableData(savedVisible, { getValue: sortValue });
  const visibleRows = [...sortedRows, ...rows.filter((r) => r.isNew)];
  const isFiltered = Object.values(filters).some((text) => text.trim() !== "");

  const groupOptions = useMemo(() => uniqueSorted(rows.map((r) => r.group_name)), [rows]);
  const barcaOptions = useMemo(() => uniqueSorted([...barcaNames, ...rows.map((r) => r.barca)]), [barcaNames, rows]);
  const { sum, isValid } = validateRows(rows);

  const updateRow = (id, field, value) => {
    setRows((prev) => prev.map((r) => (r._id === id ? { ...r, [field]: value } : r)));
    setSaveSuccess(false);
  };
  const addRow = () => setRows((prev) => [...prev, withId(blankRow())]);
  const dropRow = (id) => {
    setRows((prev) => prev.filter((r) => r._id !== id));
    setSaveSuccess(false);
  };
  // After Corrigir: a position holding something again can't be (or stay)
  // removed — the server already brought it back.
  const correctedRow = (id, quantity) =>
    setRows((prev) =>
      prev.map((r) =>
        r._id !== id
          ? r
          : quantity > 0
            ? { ...r, current_quantity: quantity, removed: false, markedForRemoval: false, markedForRestore: false }
            : { ...r, current_quantity: quantity }
      )
    );
  const setFilter = (key, text) => setFilters((prev) => ({ ...prev, [key]: text }));

  const handleSave = async () => {
    setSaving(true);
    setSaveError("");
    try {
      await save(toSaveRequest(rows));
      setSaveSuccess(true);
    } catch (err) {
      setSaveError(err.message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 1 }}>
        Portfolio Targets
      </Typography>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: 2, maxWidth: 640 }}>
        Every row's target % — across the whole portfolio, every asset class together — must sum to 100%.
        Quantities come from CSV import and are read-only; a new asset (added below) can have a starting
        quantity. A single-source row can have its quantity fixed with "Corrigir" (applied immediately); a
        row that sums several sources shows "multi-fonte". A row holding nothing can be removed on Save: it
        disappears from every page (its ledger history is kept) and comes back by itself if it gets a quantity
        again.
      </Typography>

      {error && (
        <Typography variant="body2" sx={{ color: "error.main", mb: 2 }}>
          {error}
        </Typography>
      )}

      {removedCount > 0 && (
        <FormControlLabel
          sx={{ mb: 1 }}
          control={<Switch size="small" checked={showRemoved} onChange={(e) => setShowRemoved(e.target.checked)} />}
          label={`Show removed (${removedCount})`}
        />
      )}

      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto", mb: 2 }}>
        <Table sx={{ minWidth: 960 }}>
          <SortableTableHead
            columns={COLUMNS}
            sortConfig={sortConfig}
            onSort={toggleSort}
            filters={filters}
            onFilterChange={setFilter}
            filterRowEnd={
              <Button size="small" disabled={!isFiltered} onClick={() => setFilters(NO_FILTERS)}>
                Clear
              </Button>
            }
          />
          <TableBody>
            {visibleRows.map((r) => (
              <TableRow key={r._id} hover sx={{ ...zebraRowSx, opacity: isKept(r) ? 1 : 0.55 }}>
                <TableCell>
                  <TextField
                    size="small"
                    value={r.symbol}
                    slotProps={{ htmlInput: { style: { textDecoration: isKept(r) ? "none" : "line-through" } } }}
                    onChange={(e) => updateRow(r._id, "symbol", e.target.value)}
                    disabled={!r.isNew}
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
                    onInputChange={(_, value) => updateRow(r._id, "group_name", value)}
                    disabled={!r.isNew}
                    renderInput={(params) => <TextField {...params} sx={{ width: 120 }} />}
                  />
                </TableCell>
                <TableCell>
                  <Autocomplete
                    freeSolo
                    size="small"
                    options={barcaOptions}
                    value={r.barca || ""}
                    onInputChange={(_, value) => updateRow(r._id, "barca", value)}
                    disabled={!r.isNew}
                    renderInput={(params) => <TextField {...params} sx={{ width: 140 }} />}
                  />
                </TableCell>
                <TableCell>
                  <FormControl size="small" sx={{ minWidth: 130 }} disabled={!r.isNew}>
                    <Select
                      value={r.asset_class}
                      onChange={(e) => updateRow(r._id, "asset_class", e.target.value)}
                      renderValue={(c) => c}
                      inputProps={{ "aria-label": "Asset class" }}
                    >
                      {ASSET_CLASSES.map((c) => (
                        <MenuItem key={c} value={c}>
                          <ListItemText primary={c} secondary={ASSET_CLASS_HINTS[c]} />
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
                    onChange={(e) => updateRow(r._id, "target_percent", e.target.value)}
                    disabled={!isKept(r)}
                    sx={{ width: 90 }}
                    slotProps={{ htmlInput: { step: 0.1, min: 0, max: 100 } }}
                  />
                </TableCell>
                <TableCell align="right">
                  <QuantityCell
                    row={r}
                    onChange={(value) => updateRow(r._id, "current_quantity", value)}
                    // Update only this row: re-fetching everything would wipe
                    // unsaved Target % edits on other rows.
                    onCorrected={(quantity) => correctedRow(r._id, quantity)}
                  />
                </TableCell>
                <TableCell>
                  <TextField
                    size="small"
                    value={r.notes || ""}
                    onChange={(e) => updateRow(r._id, "notes", e.target.value)}
                    disabled={!r.isNew}
                    sx={{ width: 140 }}
                  />
                </TableCell>
                <TableCell align="right">
                  <TargetRowActions row={r} onDrop={() => dropRow(r._id)} onMark={(field, value) => updateRow(r._id, field, value)} />
                </TableCell>
              </TableRow>
            ))}
            {!loading && rows.length === 0 && (
              <TableRow>
                <TableCell colSpan={COLUMNS.length} align="center" sx={{ color: "text.secondary" }}>
                  No portfolio targets yet.
                </TableCell>
              </TableRow>
            )}
            {rows.length > 0 && visibleRows.length === 0 && (
              <TableRow>
                <TableCell colSpan={COLUMNS.length} align="center" sx={{ color: "text.secondary" }}>
                  {isFiltered ? "No rows match the filters." : "Every position is removed. Turn on “Show removed” to see them."}
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
        {isFiltered && (
          <Typography variant="body2" sx={{ color: "text.secondary" }}>
            Showing {sortedRows.length} of {rows.filter((r) => !r.isNew).length} rows · filtered sum{" "}
            {getTotalTargetPercent(sortedRows.filter(isKept)).toFixed(2)}% (Sum and Save All cover every row)
          </Typography>
        )}
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
