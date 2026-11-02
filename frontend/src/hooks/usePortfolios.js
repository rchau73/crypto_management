import { useCallback, useEffect, useState } from "react";
import { fetchPortfolios } from "../api/client";
import { loadRememberedPortfolioId, pickPortfolio, rememberPortfolioId } from "../utils/portfolios";

// The portfolios the logged-in user can open, and which one is selected.
// The selection is remembered per browser; a remembered id the user can no
// longer open falls back to the first portfolio.
export function usePortfolios() {
  const [portfolios, setPortfolios] = useState([]);
  const [selectedId, setSelectedId] = useState(loadRememberedPortfolioId);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // Re-reads the list (after creating or renaming one). `thenSelect` picks
  // a portfolio from the new list, e.g. the one just created.
  const reload = useCallback(async (thenSelect) => {
    try {
      setPortfolios(await fetchPortfolios());
      setError("");
      if (thenSelect !== undefined) {
        setSelectedId(thenSelect);
        rememberPortfolioId(thenSelect);
      }
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const select = useCallback((id) => {
    setSelectedId(id);
    rememberPortfolioId(id);
  }, []);

  return { portfolios, current: pickPortfolio(portfolios, selectedId), select, reload, loading, error };
}
