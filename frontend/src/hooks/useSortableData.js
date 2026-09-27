import { useMemo, useState } from "react";

// Generic click-to-sort behavior for a table. `getValue(row, key)` lets a
// caller sort on computed columns (e.g. a deviation derived from filter
// state) instead of just `row[key]`; it defaults to plain property access.
export function useSortableData(rows, { initialKey = null, initialDirection = "asc", getValue } = {}) {
  const [sortConfig, setSortConfig] = useState({ key: initialKey, direction: initialDirection });

  const toggleSort = (key) => {
    setSortConfig((prev) =>
      prev.key === key ? { key, direction: prev.direction === "asc" ? "desc" : "asc" } : { key, direction: "asc" }
    );
  };

  const sortedRows = useMemo(() => {
    if (!sortConfig.key) return rows;
    const extract = getValue || ((row, key) => row[key]);
    return [...rows].sort((a, b) => {
      const aValue = extract(a, sortConfig.key);
      const bValue = extract(b, sortConfig.key);
      if (aValue === undefined || bValue === undefined) return 0;
      if (typeof aValue === "number" && typeof bValue === "number") {
        return sortConfig.direction === "asc" ? aValue - bValue : bValue - aValue;
      }
      return sortConfig.direction === "asc"
        ? String(aValue).localeCompare(String(bValue))
        : String(bValue).localeCompare(String(aValue));
    });
  }, [rows, sortConfig, getValue]);

  return { sortConfig, toggleSort, sortedRows };
}
