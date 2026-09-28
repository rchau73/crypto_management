import { useCallback, useEffect, useState } from "react";
import { fetchPortfolioTargets, savePortfolioTargets } from "../api/client";

// Manager+ editable-table data for the portfolio-targets admin tab.
// `active` should be false while that tab isn't visible, matching the
// pattern used for the Admin tab's user list.
export function usePortfolioTargets(active) {
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
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    if (active) refresh();
  }, [active, refresh]);

  const save = useCallback(
    async (nextRows) => {
      await savePortfolioTargets(nextRows);
      await refresh();
    },
    [refresh]
  );

  return { rows, loading, error, refresh, save };
}
