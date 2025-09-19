import { useCallback, useState } from "react";

const DEFAULT_API_URL = "http://localhost:3001";
const apiUrl = (import.meta.env.VITE_API_URL ?? DEFAULT_API_URL).replace(/\/$/, "");
const ALLOCATIONS_ENDPOINT = `${apiUrl}/api/allocations`;

const emptyData = {
  per_asset: [],
  per_group: [],
  per_barca: [],
  per_barca_actual: [],
};

export function useAllocationsData() {
  const [data, setData] = useState(emptyData);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [lastUpdate, setLastUpdate] = useState(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(ALLOCATIONS_ENDPOINT);
      if (!response.ok) {
        throw new Error(`Request failed with status ${response.status}`);
      }

      const payload = await response.json();
      setData({
        per_asset: payload.per_asset ?? [],
        per_group: payload.per_group ?? [],
        per_barca: payload.per_barca ?? [],
        per_barca_actual: payload.per_barca_actual ?? [],
      });
      setLastUpdate(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const clearError = useCallback(() => setError(null), []);

  return {
    allocations: data.per_asset,
    groupAllocations: data.per_group,
    barcaAllocations: data.per_barca,
    barcaActualAllocations: data.per_barca_actual,
    loading,
    error,
    lastUpdate,
    refresh,
    clearError,
  };
}
