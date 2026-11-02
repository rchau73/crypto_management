import { useCallback, useEffect, useState } from "react";
import { fetchBarcaTargets, saveBarcaTargets } from "../api/client";
import { useCurrentPortfolio } from "./useCurrentPortfolio";

// Manager+ editable-table data for the BARCA-targets admin tab, for the
// current portfolio and one market profile at a time (see migrations/0006 — a barca target
// belongs to exactly one market, with no cross-market fallback).
export function useBarcaTargets(market) {
  const { id: portfolioId } = useCurrentPortfolio();
  const [targets, setTargets] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setTargets(await fetchBarcaTargets(portfolioId, market));
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, [portfolioId, market]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const save = useCallback(
    async (nextTargets) => {
      await saveBarcaTargets(portfolioId, market, nextTargets);
      await refresh();
    },
    [portfolioId, market, refresh]
  );

  return { targets, loading, error, refresh, save };
}
