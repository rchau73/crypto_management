import { useEffect, useMemo, useState } from "react";
import { fetchHistory } from "../api/client";
import { computeYAxisDomain } from "../utils/allocationMath";
import { groupHistoryByPeriod, parseHistoryRows, topSeriesByValue } from "../utils/historyBucketing";

const MULTI_SERIES_LEVELS = new Set(["assets", "barca", "groups"]);
const DEFAULT_SERIES_LIMIT = 5;

// Owns the historical dashboard: which level/granularity is selected, the
// bucketed chart data, and the fetch that refreshes it. `active` should be
// false while the Dashboard tab isn't visible, so we don't fetch history the
// user hasn't asked to see yet.
export function useHistoryDashboard(active) {
  const [level, setLevel] = useState("totals");
  const [granularity, setGranularity] = useState("daily");
  const [series, setSeries] = useState([]);
  const [selectedSeries, setSelectedSeries] = useState([]);
  const [data, setData] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [raw, setRaw] = useState(null);

  const refresh = async (levelOverride = level) => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetchHistory(levelOverride);
      setRaw(response);
      const parsed = parseHistoryRows(levelOverride, response.rows || []);
      const bucketed = groupHistoryByPeriod(levelOverride, parsed, granularity);
      setData(bucketed.data);
      if (MULTI_SERIES_LEVELS.has(levelOverride)) {
        setSeries(bucketed.series);
        setSelectedSeries((prev) =>
          prev.length === 0 && bucketed.series.length > 0
            ? topSeriesByValue(bucketed.data, bucketed.series, DEFAULT_SERIES_LIMIT)
            : prev
        );
      }
    } catch (err) {
      console.error("Failed to fetch history", err);
      setError(String(err));
      setData([]);
    }
    setLoading(false);
  };

  useEffect(() => {
    if (active) refresh(level);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [level, granularity, active]);

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
    raw,
    yAxisDomain,
    refresh,
  };
}
