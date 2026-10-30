import { useEffect, useMemo, useRef, useState } from "react";
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
  TableRow,
  TextField,
  Typography,
} from "@mui/material";
import { usePortfolioTargets } from "../../hooks/usePortfolioTargets";
import { useSortableData } from "../../hooks/useSortableData";
import { SortableTableHead } from "../SortableTableHead";
import { fetchBarcaTargets } from "../../api/client";
import { zebraRowSx } from "../../styles/tableStyles";
import { STATUS_COLORS } from "../../theme";
import { QuantityCell } from "../QuantityCell";
import {
  ASSET_CLASSES,
  blankRow,
  filterRows,
  sortValue,
  toSavePayload,
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

// BARCA names from both market profiles, used only as typing suggestions.
function useBarcaNameSuggestions() {
  const [names, setNames] = useState([]);
  useEffect(() => {
    Promise.all([fetchBarcaTargets("BullMarket"), fetchBarcaTargets("BearMarket")])
      .then(([bull, bear]) => setNames(uniqueSorted([...bull, ...bear].map((t) => t.barca))))
      .catch(() => setNames([]));
  }, []);
  return names;
}

// Manager+ tab: edit every position's target % (all asset classes together,
// must sum to 100%) and save them in one go with "Save All".
//
// For an existing row, only the target is editable. Its symbol/group/BARCA/
// asset class identify its holdings in the ledger — renaming would move the
// target away from the holdings — so to "rename", remove the row and add a
// new one. Quantity rules live in QuantityCell.
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
  const barcaNames = useBarcaNameSuggestions();
  const nextId = useRef(0);
  const withId = (row) => ({ ...row, _id: nextId.current++ });

  useEffect(() => {
    setRows(savedRows.map((r) => withId({ ...r, isNew: false })));
    setSaveSuccess(false);
  }, [savedRows]);

  const savedVisible = useMemo(() => filterRows(rows.filter((r) => !r.isNew), filters), [rows, filters]);
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
  const removeRow = (id) => {
    setRows((prev) => prev.filter((r) => r._id !== id));
    setSaveSuccess(false);
  };
  const setFilter = (key, text) => setFilters((prev) => ({ ...prev, [key]: text }));

  const handleSave = async () => {
    setSaving(true);
    setSaveError("");
    try {
      await save(toSavePayload(rows));
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
        row that sums several sources shows "multi-fonte". Remove takes the row's target out of the portfolio
        on Save (its ledger history is kept).
      </Typography>

      {error && (
        <Typography variant="body2" sx={{ color: "error.main", mb: 2 }}>
          {error}
        </Typography>
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
              <TableRow key={r._id} hover sx={zebraRowSx}>
                <TableCell>
                  <TextField
                    size="small"
                    value={r.symbol}
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
                    <Select value={r.asset_class} onChange={(e) => updateRow(r._id, "asset_class", e.target.value)}>
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
                    onChange={(e) => updateRow(r._id, "target_percent", e.target.value)}
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
                    onCorrected={(quantity) => updateRow(r._id, "current_quantity", quantity)}
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
                  <Button size="small" color="error" onClick={() => removeRow(r._id)}>
                    Remove
                  </Button>
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
                  No rows match the filters.
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
            {getTotalTargetPercent(sortedRows).toFixed(2)}% (Sum and Save All cover every row)
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
