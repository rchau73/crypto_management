import { Box, Paper, Table, TableBody, TableCell, TableContainer, TableRow, Typography } from "@mui/material";
import { SortableTableHead } from "../SortableTableHead";
import { PaginationControls } from "../PaginationControls";
import { DeviationBadge } from "../DeviationBadge";
import { useSortableData } from "../../hooks/useSortableData";
import { usePagination } from "../../hooks/usePagination";
import { deriveRowMetrics, deviationSeverity, getTotalTargetPercent } from "../../utils/allocationMath";
import { formatNumber, formatPrice } from "../../utils/formatters";
import { compactCellSx, numericCellSx, zebraRowSx } from "../../styles/tableStyles";

const COLUMNS = [
  { key: "symbol", label: "Symbol", width: "10%" },
  { key: "price", label: "Price", align: "right", width: "12%" },
  { key: "current_quantity", label: "Qty", align: "right", width: "10%" },
  { key: "value", label: "Value", align: "right", width: "14%" },
  { key: "target_percent", label: "Target %", align: "right", width: "10%" },
  { key: "current_percent", label: "Current %", align: "right", width: "10%" },
  { key: "deviation", label: "Deviation", align: "right", width: "12%" },
  { key: "value_deviation", label: "Value Deviation", align: "right", width: "12%" },
  { key: "dca", label: "DCA", align: "right", width: "10%" },
];

const COMPUTED_METRIC_KEYS = {
  current_percent: (m) => m.currentPercent,
  deviation: (m) => m.deviation,
  value_deviation: (m) => m.valueDeviation,
  dca: (m) => m.dca,
};

// Tab 1: allocations grouped by symbol only (Group/BARCA columns don't apply
// here since one symbol can span several of them).
export function PerAssetTotalTab({ rows, totalValue, isFiltered, filterKey }) {
  const getSortValue = (row, key) => {
    const metricGetter = COMPUTED_METRIC_KEYS[key];
    return metricGetter ? metricGetter(deriveRowMetrics(row, totalValue)) : row[key];
  };

  const { sortConfig, toggleSort, sortedRows } = useSortableData(rows, {
    initialKey: "value",
    initialDirection: "desc",
    getValue: getSortValue,
  });
  const { page, setPage, totalPages, paginatedRows } = usePagination(sortedRows, { resetKey: filterKey });

  if (rows.length === 0) return null;

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        Per-Asset Total Allocation (All Groups/BARCA)
        <Box component="span" sx={{ color: "primary.main", fontWeight: "normal", ml: 2, fontSize: 14 }}>
          Total Target %: {formatNumber(getTotalTargetPercent(rows))}%
          {isFiltered && (
            <Box component="span" sx={{ color: "text.secondary", ml: 1 }}>
              (Filtered Total: ${formatNumber(totalValue)})
            </Box>
          )}
        </Box>
      </Typography>
      <Typography variant="body2" sx={{ color: "text.secondary", mb: 1 }}>
        Grouped by symbol; Group and BARCA columns are ignored in this view.
      </Typography>
      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
        <Table sx={{ minWidth: 750 }}>
          <SortableTableHead columns={COLUMNS} sortConfig={sortConfig} onSort={toggleSort} />
          <TableBody>
            {paginatedRows.map((row, idx) => {
              const metrics = deriveRowMetrics(row, totalValue);
              const severity = deviationSeverity(metrics);
              return (
                <TableRow key={row.symbol || idx} hover sx={zebraRowSx}>
                  <TableCell sx={{ ...compactCellSx, whiteSpace: "nowrap" }}>{row.symbol}</TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    {formatPrice(row.price)}
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    {formatNumber(row.current_quantity)}
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    ${formatNumber(row.value)}
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    {formatNumber(row.target_percent)}%
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    {formatNumber(metrics.currentPercent)}%
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    <DeviationBadge severity={severity}>
                      {metrics.deviation > 0 ? "+" : ""}
                      {formatNumber(metrics.deviation)}%
                    </DeviationBadge>
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    {metrics.valueDeviation > 0 ? "+$" : metrics.valueDeviation < 0 ? "-$" : "$"}
                    {formatNumber(Math.abs(metrics.valueDeviation))}
                  </TableCell>
                  <TableCell align="right" sx={numericCellSx}>
                    ${formatNumber(metrics.dca)}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </TableContainer>
      <PaginationControls page={page} totalPages={totalPages} onPageChange={setPage} />
    </Box>
  );
}
