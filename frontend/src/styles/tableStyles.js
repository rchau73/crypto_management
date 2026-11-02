// Shared MUI `sx` objects for the allocation tables, so column styling is
// defined once instead of being retyped identically in every table.

// 13px: the smallest size that stays readable in a dense table (it was 10px).
export const compactCellSx = { fontSize: 13, padding: "8px 6px" };

// Numeric columns get tabular figures so digits line up vertically down the column.
export const numericCellSx = { ...compactCellSx, fontVariantNumeric: "tabular-nums" };

export const truncatedCellSx = {
  ...compactCellSx,
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
  maxWidth: "120px",
};

// Faint zebra striping — subtle enough not to fight the deviation-severity
// coloring, but enough to help track a row across an 11-column table.
export const zebraRowSx = {
  "&:nth-of-type(odd)": { backgroundColor: "rgba(255, 255, 255, 0.025)" },
};

export function headerCellSx(width) {
  return {
    cursor: "pointer",
    fontWeight: "bold",
    fontSize: 13,
    width: width || "auto",
    padding: "8px 4px",
    whiteSpace: "nowrap",
    overflow: "hidden",
    textOverflow: "ellipsis",
  };
}

export function sortButtonSx(isActive) {
  return {
    color: isActive ? "primary.main" : "inherit",
    minWidth: 0,
    fontWeight: "bold",
    fontSize: 13,
    textTransform: "none",
    p: 0,
  };
}
