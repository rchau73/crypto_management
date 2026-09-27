import { useEffect, useMemo, useState } from "react";

// Generic page-slicing for a table. Pass a `resetKey` that changes whenever
// the underlying filter changes (e.g. a join of the active filter values) so
// the view snaps back to page 1 instead of showing an empty or mismatched page.
export function usePagination(rows, { pageSize = 10, resetKey } = {}) {
  const [page, setPage] = useState(1);
  const totalPages = Math.max(1, Math.ceil(rows.length / pageSize));

  useEffect(() => {
    setPage(1);
  }, [resetKey]);

  useEffect(() => {
    if (page > totalPages) setPage(totalPages);
  }, [page, totalPages]);

  const paginatedRows = useMemo(() => rows.slice((page - 1) * pageSize, page * pageSize), [rows, page, pageSize]);

  return { page, setPage, totalPages, paginatedRows };
}
