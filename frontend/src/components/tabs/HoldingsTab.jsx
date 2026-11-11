import { useMemo } from "react";
import {
  Box,
  Button,
  InputAdornment,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableRow,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { SortableTableHead } from "../SortableTableHead";
import { UnpricedAlert } from "../UnpricedAlert";
import { DeviationBadge } from "../DeviationBadge";
import { useSortableData } from "../../hooks/useSortableData";
import { DEFAULT_VIEW, VIEWS, buildHoldingRows, filterHoldingRows } from "../../utils/holdings";
import { formatNumber, formatPrice } from "../../utils/formatters";
import { matchUnpriced } from "../../utils/unpriced";
import { compactCellSx, numericCellSx, truncatedCellSx, zebraRowSx } from "../../styles/tableStyles";
import { STATUS_COLORS } from "../../theme";

const TARGET_SUM_TOLERANCE = 0.01;

const STATUS_LABEL = { above: "Above band", below: "Below band", in: "In band" };
const STATUS_ORDER = { above: 2, below: 1, in: 0 };

// Columns every view shares, after the view-specific ones.
const METRIC_COLUMNS = [
  { key: "value", label: "Value", align: "right" },
  { key: "currentPercent", label: "Current %", align: "right" },
  { key: "targetPercent", label: "Target %", align: "right" },
  { key: "deviation", label: "Deviation", align: "right" },
  { key: "gap", label: "$ gap", align: "right" },
  { key: "status", label: "Band" },
  { key: "step", label: "DCA step", align: "right" },
];

function columnsFor(view) {
  const name = { key: "name", label: VIEWS.find((v) => v.key === view).nameLabel };
  if (view === "position") {
    return [
      name,
      { key: "group", label: "Group" },
      { key: "barca", label: "BARCA" },
      { key: "price", label: "Price", align: "right" },
      { key: "quantity", label: "Qty", align: "right" },
      ...METRIC_COLUMNS,
    ];
  }
  if (view === "asset") {
    return [
      name,
      { key: "barca", label: "BARCA" },
      { key: "price", label: "Price", align: "right" },
      { key: "quantity", label: "Qty", align: "right" },
      ...METRIC_COLUMNS,
    ];
  }
  return [name, ...METRIC_COLUMNS];
}

// Gap and DCA step sort by size, whichever side of the target they are on:
// the biggest move to make comes first either way.
function sortValue(row, key) {
  if (key === "gap") return Math.abs(row.gap);
  if (key === "step") return row.step?.amount ?? 0;
  if (key === "status") return STATUS_ORDER[row.status];
  return row[key];
}

function signedMoney(n) {
  const rounded = Math.round(n * 100) / 100;
  const sign = rounded > 0 ? "+" : rounded < 0 ? "−" : "";
  return `${sign}$${formatNumber(Math.abs(n))}`;
}

function signedPoints(n) {
  const rounded = Math.round(n * 100) / 100;
  const sign = rounded > 0 ? "+" : rounded < 0 ? "−" : "";
  return `${sign}${formatNumber(Math.abs(n))} pts`;
}

function Cell({ column, row }) {
  switch (column.key) {
    case "name":
      return (
        <TableCell component="th" scope="row" sx={{ ...compactCellSx, fontWeight: 700, whiteSpace: "nowrap" }}>
          {row.name}
        </TableCell>
      );
    case "group":
    case "barca":
      return (
        <TableCell sx={truncatedCellSx} title={row[column.key]}>
          {row[column.key]}
        </TableCell>
      );
    case "price":
      return <TableCell align="right" sx={numericCellSx}>${formatPrice(row.price)}</TableCell>;
    case "quantity":
      return <TableCell align="right" sx={numericCellSx}>{formatNumber(row.quantity)}</TableCell>;
    case "value":
      return <TableCell align="right" sx={numericCellSx}>${formatNumber(row.value)}</TableCell>;
    case "currentPercent":
    case "targetPercent":
      return <TableCell align="right" sx={numericCellSx}>{formatNumber(row[column.key])}%</TableCell>;
    case "deviation":
      return <TableCell align="right" sx={numericCellSx}>{signedPoints(row.deviation)}</TableCell>;
    case "gap":
      return <TableCell align="right" sx={numericCellSx}>{signedMoney(row.gap)}</TableCell>;
    case "status":
      return (
        <TableCell sx={{ ...compactCellSx, whiteSpace: "nowrap" }}>
          <DeviationBadge status={row.status}>{STATUS_LABEL[row.status]}</DeviationBadge>
        </TableCell>
      );
    case "step": {
      // Rule 1: only out-of-band rows call for action; the step of an
      // in-band row is shown dimmed, for reference.
      const active = row.status !== "in";
      return (
        <TableCell
          align="right"
          sx={{ ...numericCellSx, whiteSpace: "nowrap", fontWeight: active ? 700 : 400, color: active ? "text.primary" : "text.secondary" }}
        >
          {row.step ? `${row.step.verb} $${formatNumber(row.step.amount)}` : "—"}
        </TableCell>
      );
    }
    default:
      return <TableCell sx={compactCellSx}>{row[column.key]}</TableCell>;
  }
}

// Holdings: every position against its target, viewed by position, asset,
// group or BARCA (it replaces the four separate table tabs). The view, the
// text filter and "outside band only" live in the URL (`params`), so the
// header search can link straight to "ETH in the Asset view".
export function HoldingsTab({ report, params, onParamsChange }) {
  const view = VIEWS.some((v) => v.key === params.view) ? params.view : DEFAULT_VIEW;
  const query = params.q || "";
  const outOfBandOnly = params.out === "1";

  const { rows } = useMemo(() => buildHoldingRows(view, report), [view, report]);
  const filtered = useMemo(() => filterHoldingRows(rows, { query, outOfBandOnly }), [rows, query, outOfBandOnly]);
  // One sort for all views: the metric columns are shared, so the order
  // you picked survives switching view.
  const { sortConfig, toggleSort, sortedRows } = useSortableData(filtered, {
    initialKey: "gap",
    initialDirection: "desc",
    getValue: sortValue,
  });

  const unpriced = report.unpriced || [];
  const unpricedMatches = matchUnpriced(unpriced, query);
  const columns = columnsFor(view);
  const outCount = rows.filter((r) => r.status !== "in").length;
  const targetSum = rows.reduce((sum, r) => sum + r.targetPercent, 0);
  const targetSumOff = rows.length > 0 && Math.abs(targetSum - 100) > TARGET_SUM_TOLERANCE;

  const update = (changes, options) => onParamsChange({ view, q: query, out: outOfBandOnly ? "1" : "", ...changes }, options);

  return (
    <Box>
      <Stack direction="row" useFlexGap spacing={2} sx={{ mb: 2, flexWrap: "wrap", alignItems: "center" }}>
        <Typography variant="h5" component="h1" sx={{ fontWeight: 700, mr: "auto" }}>
          Holdings
        </Typography>
        <ToggleButtonGroup
          exclusive
          size="small"
          value={view}
          onChange={(_, next) => next && update({ view: next })}
          aria-label="View by"
        >
          {VIEWS.map((v) => (
            <ToggleButton key={v.key} value={v.key} sx={{ px: 2, minHeight: 40 }}>
              {v.label}
            </ToggleButton>
          ))}
        </ToggleButtonGroup>
      </Stack>

      <Stack direction="row" useFlexGap spacing={2} sx={{ mb: 1.5, flexWrap: "wrap", alignItems: "center" }}>
        <TextField
          size="small"
          value={query}
          onChange={(e) => update({ q: e.target.value }, { replace: true })}
          placeholder="Filter by name, group or BARCA"
          slotProps={{
            htmlInput: { "aria-label": "Filter rows" },
            input: query
              ? {
                  endAdornment: (
                    <InputAdornment position="end">
                      <Button size="small" onClick={() => update({ q: "" })}>
                        Clear
                      </Button>
                    </InputAdornment>
                  ),
                }
              : undefined,
          }}
          sx={{ flex: "1 1 240px", maxWidth: 360 }}
        />
        <ToggleButton
          value="out"
          size="small"
          selected={outOfBandOnly}
          onChange={() => update({ out: outOfBandOnly ? "" : "1" })}
          sx={{ minHeight: 40, px: 2 }}
        >
          Only outside band ({outCount})
        </ToggleButton>
      </Stack>

      <Typography variant="body2" sx={{ color: "text.secondary", mb: 1.5 }}>
        {outCount} of {rows.length} outside their band (target ± 20% of target). DCA step = 30% of the $ gap.
        {targetSumOff && (
          <Box component="span" sx={{ color: STATUS_COLORS.warning, fontWeight: 700, ml: 1 }}>
            Targets in this view add up to {formatNumber(targetSum)}%, not 100%.
          </Box>
        )}
      </Typography>

      <UnpricedAlert unpriced={unpriced} />

      <TableContainer component={Paper} sx={{ maxHeight: "70vh", overflowX: "auto" }}>
        <Table stickyHeader size="small" sx={{ minWidth: view === "position" ? 1100 : 900 }}>
          <SortableTableHead columns={columns} sortConfig={sortConfig} onSort={toggleSort} />
          <TableBody>
            {sortedRows.map((row) => (
              <TableRow key={row.id} hover sx={zebraRowSx}>
                {columns.map((col) => (
                  <Cell key={col.key} column={col} row={row} />
                ))}
              </TableRow>
            ))}
            {sortedRows.length === 0 && (
              <TableRow>
                <TableCell colSpan={columns.length} sx={{ py: 4, textAlign: "center", color: "text.secondary" }}>
                  No rows match{query ? ` “${query}”` : ""}
                  {outOfBandOnly ? " outside their band" : ""}.{" "}
                  {unpricedMatches.length > 0 &&
                    `${unpricedMatches.map((u) => u.symbol).join(", ")} ${unpricedMatches.length === 1 ? "is" : "are"} held but ${
                      unpricedMatches.length === 1 ? "has" : "have"
                    } no price (see the warning above). `}
                  <Button size="small" onClick={() => update({ q: "", out: "" })}>
                    Show all rows
                  </Button>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </TableContainer>
    </Box>
  );
}
