import { useCallback, useEffect, useState } from "react";
import { fetchPortfolioTargets, savePortfolioTargets } from "../api/client";
import { useCurrentPortfolio } from "./useCurrentPortfolio";

// Manager+ editable-table data for the Portfolio Targets tab, for the
// current portfolio, removed positions included (the tab can show them).
// Fetches when the tab mounts.
export function usePortfolioTargets() {
  const { id: portfolioId } = useCurrentPortfolio();
  const [rows, setRows] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setRows(await fetchPortfolioTargets(portfolioId, { includeRemoved: true }));
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, [portfolioId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const save = useCallback(
    // `request` is { rows, remove } (see toSaveRequest).
    async (request) => {
      await savePortfolioTargets(portfolioId, request);
      await refresh();
    },
    [portfolioId, refresh]
  );

  return { rows, loading, error, refresh, save };
}
