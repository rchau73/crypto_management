import { useCallback, useEffect, useState } from "react";
import { fetchPortfolioTargets, savePortfolioTargets } from "../api/client";

// Manager+ editable-table data for the Portfolio Targets tab. Fetches when
// the tab mounts.
export function usePortfolioTargets() {
  const [rows, setRows] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setRows(await fetchPortfolioTargets());
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const save = useCallback(
    async (nextRows) => {
      await savePortfolioTargets(nextRows);
      await refresh();
    },
    [refresh]
  );

  return { rows, loading, error, refresh, save };
}
