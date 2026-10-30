import { Button, TableCell, TableHead, TableRow, TextField } from "@mui/material";
import { headerCellSx, sortButtonSx } from "../styles/tableStyles";

// columns: [{ key, label, align?, width?, sortable? (default true), filterable? }]
//
// Pass `filters` + `onFilterChange` to get a second header row with one
// "contains" text box per filterable column. `filterRowEnd` fills the cell of
// the last non-filterable column (e.g. a "Clear" button).
export function SortableTableHead({ columns, sortConfig, onSort, filters, onFilterChange, filterRowEnd }) {
  return (
    <TableHead>
      <TableRow>
        {columns.map((col) =>
          col.sortable === false ? (
            <TableCell key={col.key} align={col.align || "left"} sx={{ ...headerCellSx(col.width), cursor: "default" }}>
              {col.label}
            </TableCell>
          ) : (
            <TableCell key={col.key} align={col.align || "left"} sx={headerCellSx(col.width)} onClick={() => onSort(col.key)}>
              <Button size="small" variant="text" sx={sortButtonSx(sortConfig.key === col.key)}>
                {col.label}
                {sortConfig.key === col.key ? (sortConfig.direction === "asc" ? " ▲" : " ▼") : ""}
              </Button>
            </TableCell>
          )
        )}
      </TableRow>
      {filters && (
        <TableRow>
          {columns.map((col) => (
            <TableCell key={col.key} align={col.align || "left"} sx={{ py: 0.5, px: 0.5 }}>
              {col.filterable ? (
                <TextField
                  size="small"
                  placeholder="Filter"
                  value={filters[col.key] || ""}
                  onChange={(e) => onFilterChange(col.key, e.target.value)}
                  slotProps={{ htmlInput: { "aria-label": `Filter ${col.label}` } }}
                  sx={{ width: "100%", minWidth: 70 }}
                />
              ) : (
                filterRowEnd
              )}
            </TableCell>
          ))}
        </TableRow>
      )}
    </TableHead>
  );
}
