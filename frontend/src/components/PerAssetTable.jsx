import { useEffect, useMemo, useState } from "react";
import {
  Box,
  Button,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";

import { formatNumber, sumTargetPercent } from "../utils/format";

const columns = [
  { key: "symbol", label: "Symbol" },
  { key: "group", label: "Group" },
  { key: "barca", label: "BARCA" },
  { key: "price", label: "Price", align: "right" },
  { key: "current_quantity", label: "Qty", align: "right" },
  { key: "value", label: "Value", align: "right" },
  { key: "target_percent", label: "Target %", align: "right" },
  { key: "current_percent", label: "Current %", align: "right" },
  { key: "deviation", label: "Deviation", align: "right" },
  { key: "value_deviation", label: "Value Deviation", align: "right" },
  { key: "dca", label: "DCA", align: "right" },
];

const pageSize = 10;

export function PerAssetTable({ rows }) {
  const [page, setPage] = useState(1);
  const [sortConfig, setSortConfig] = useState({ key: "value", direction: "desc" });

  useEffect(() => {
    setPage(1);
  }, [rows]);

  const sortedRows = useMemo(() => {
    if (!sortConfig.key) return rows;
    return [...rows].sort((a, b) => {
      const aValue = a[sortConfig.key];
      const bValue = b[sortConfig.key];

      if (aValue == null && bValue == null) return 0;
      if (aValue == null) return 1;
      if (bValue == null) return -1;

      if (typeof aValue === "number" && typeof bValue === "number") {
        return sortConfig.direction === "asc" ? aValue - bValue : bValue - aValue;
      }

      const aStr = String(aValue).toLowerCase();
      const bStr = String(bValue).toLowerCase();
      return sortConfig.direction === "asc"
        ? aStr.localeCompare(bStr)
        : bStr.localeCompare(aStr);
    });
  }, [rows, sortConfig]);

  const totalPages = Math.max(1, Math.ceil(sortedRows.length / pageSize));
  const paginatedRows = useMemo(
    () => sortedRows.slice((page - 1) * pageSize, page * pageSize),
    [sortedRows, page]
  );

  const handleSort = (key) => {
    setSortConfig((prev) => {
      if (prev.key === key) {
        return { key, direction: prev.direction === "asc" ? "desc" : "asc" };
      }
      return { key, direction: "asc" };
    });
  };

  return (
    <Box>
      <Typography variant="h6" sx={{ mt: 2, mb: 2 }}>
        Per-Asset Allocation (Current Prices)
        <span
          style={{
            color: "#00C49F",
            fontWeight: "normal",
            marginLeft: 16,
            fontSize: 14,
          }}
        >
          Total Target %: {formatNumber(sumTargetPercent(sortedRows))}%
        </span>
      </Typography>

      <TableContainer component={Paper} sx={{ maxWidth: "100%", overflowX: "auto" }}>
        <Table sx={{ minWidth: 900 }}>
          <TableHead>
            <TableRow>
              {columns.map((column) => (
                <TableCell
                  key={column.key}
                  align={column.align ?? "left"}
                  sx={{ cursor: "pointer", fontWeight: "bold", fontSize: 12 }}
                  onClick={() => handleSort(column.key)}
                >
                  <Button
                    size="small"
                    variant="text"
                    sx={{
                      color: sortConfig.key === column.key ? "#00C49F" : "inherit",
                      minWidth: 0,
                      fontWeight: "bold",
                      fontSize: 12,
                      textTransform: "none",
                      p: 0,
                    }}
                  >
                    {column.label}
                    {sortConfig.key === column.key
                      ? sortConfig.direction === "asc"
                        ? " ▲"
                        : " ▼"
                      : ""}
                  </Button>
                </TableCell>
              ))}
            </TableRow>
          </TableHead>
          <TableBody>
            {paginatedRows.map((row, index) => {
              const safeCurrentPercent = row.current_percent ?? 0;
              const proportionalValue = safeCurrentPercent
                ? (row.value * row.target_percent) / safeCurrentPercent
                : 0;
              const deviation = row.deviation ?? 0;
              const valueDeviation = safeCurrentPercent
                ? Math.abs(
                    deviation >= 0
                      ? row.value - proportionalValue
                      : proportionalValue - row.value
                  )
                : 0;
              const dca = valueDeviation * 0.3;

              return (
                <TableRow key={`${row.symbol}-${row.group}-${row.barca}-${index}`}>
                  <TableCell sx={{ fontSize: 10 }}>{row.symbol}</TableCell>
                  <TableCell sx={{ fontSize: 10 }}>{row.group}</TableCell>
                  <TableCell sx={{ fontSize: 10 }}>{row.barca}</TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    {formatNumber(row.price)}
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    {formatNumber(row.current_quantity)}
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    ${formatNumber(row.value)}
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    {formatNumber(row.target_percent)}%
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    {formatNumber(row.current_percent)}%
                  </TableCell>
                  <TableCell
                    align="right"
                    sx={{
                      color: Math.abs(row.deviation) > 1 ? "error.main" : "inherit",
                      fontWeight: Math.abs(row.deviation) > 1 ? "bold" : "normal",
                      fontSize: 10,
                      whiteSpace: "nowrap",
                    }}
                  >
                    {row.deviation > 0 ? "+" : ""}
                    {formatNumber(row.deviation)}%
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    ${formatNumber(valueDeviation)}
                  </TableCell>
                  <TableCell align="right" sx={{ fontSize: 10 }}>
                    ${formatNumber(dca)}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </TableContainer>

      <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", mt: 2 }}>
        <Button
          variant="outlined"
          size="small"
          onClick={() => setPage((prev) => Math.max(1, prev - 1))}
          disabled={page === 1}
          sx={{ mr: 1 }}
        >
          Prev
        </Button>
        <Typography sx={{ fontSize: 12 }}>
          Page {page} of {totalPages}
        </Typography>
        <Button
          variant="outlined"
          size="small"
          onClick={() => setPage((prev) => Math.min(totalPages, prev + 1))}
          disabled={page === totalPages}
          sx={{ ml: 1 }}
        >
          Next
        </Button>
      </Box>
    </Box>
  );
}
