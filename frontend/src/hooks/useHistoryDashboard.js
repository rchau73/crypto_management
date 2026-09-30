import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fetchHistory } from "../api/client";
import { computeYAxisDomain } from "../utils/allocationMath";
import { groupHistoryByPeriod, parseHistoryRows, topSeriesByValue } from "../utils/historyBucketing";

const MULTI_SERIES_LEVELS = new Set(["assets", "barca", "groups"]);
const DEFAULT_SERIES_LIMIT = 5;

// Owns the historical dashboard: which level/granularity is selected, the
// fetched rows, and the chart data bucketed from them. Fetches when the
// Dashboard tab mounts and whenever the level changes.
export function useHistoryDashboard() {
  const [level, setLevel] = useState("totals");
  const [granularity, setGranularity] = useState("daily");
  const [raw, setRaw] = useState(null);
  const [selectedSeries, setSelectedSeries] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  // Only the latest request may update state: switching level quickly must
  // not let a slower, older response overwrite the newer one.
  const latestRequest = useRef(0);

  const refresh = useCallback(async () => {
    const requestId = ++latestRequest.current;
    setLoading(true);
    setError(null);
    try {
      const response = await fetchHistory(level);
      if (requestId === latestRequest.current) setRaw({ level, response });
    } catch (err) {
      if (requestId === latestRequest.current) {
        setError(String(err.message || err));
        setRaw(null);
      }
    } finally {
      if (requestId === latestRequest.current) setLoading(false);
    }
  }, [level]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Re-bucketing is pure and local: changing granularity needs no new fetch.
  const { data, series } = useMemo(() => {
    if (!raw || raw.level !== level) return { data: [], series: [] };
    const parsed = parseHistoryRows(level, raw.response.rows || []);
    const bucketed = groupHistoryByPeriod(level, parsed, granularity);
    return {
      data: bucketed.data,
      series: MULTI_SERIES_LEVELS.has(level) ? bucketed.series : [],
    };
  }, [raw, level, granularity]);

  // A new level has different series names (e.g. assets vs. groups), so
  // start again from its biggest series instead of keeping stale names.
  useEffect(() => {
    setSelectedSeries(series.length > 0 ? topSeriesByValue(data, series, DEFAULT_SERIES_LIMIT) : []);
    // Only when the set of series changes — not on every re-bucket.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [series.join("|")]);

  const yAxisDomain = useMemo(() => {
    const keys = level === "totals" ? ["value"] : selectedSeries.length > 0 ? selectedSeries : series;
    return computeYAxisDomain(data, keys);
  }, [data, level, series, selectedSeries]);

  return {
    level,
    setLevel,
    granularity,
    setGranularity,
    series,
    selectedSeries,
    setSelectedSeries,
    data,
    loading,
    error,
    raw: raw?.response ?? null,
    yAxisDomain,
    refresh,
  };
}
