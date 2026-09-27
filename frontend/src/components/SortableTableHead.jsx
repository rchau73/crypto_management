import { Button, TableCell, TableHead, TableRow } from "@mui/material";
import { headerCellSx, sortButtonSx } from "../styles/tableStyles";

// columns: [{ key, label, align?, width? }]
export function SortableTableHead({ columns, sortConfig, onSort }) {
  return (
    <TableHead>
      <TableRow>
        {columns.map((col) => (
          <TableCell key={col.key} align={col.align || "left"} sx={headerCellSx(col.width)} onClick={() => onSort(col.key)}>
            <Button size="small" variant="text" sx={sortButtonSx(sortConfig.key === col.key)}>
              {col.label}
              {sortConfig.key === col.key ? (sortConfig.direction === "asc" ? " ▲" : " ▼") : ""}
            </Button>
          </TableCell>
        ))}
      </TableRow>
    </TableHead>
  );
}
