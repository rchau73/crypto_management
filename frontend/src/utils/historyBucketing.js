// Turns raw /api/history rows into the per-period series the dashboard chart
// renders. Kept free of React so the bucketing logic (the trickiest part of
// the dashboard) can be unit-tested without mounting anything.
import dayjs from "dayjs";

function toNumber(raw) {
  const value = typeof raw === "number" ? raw : Number(raw || 0);
  return isNaN(value) ? 0 : value;
}

export function parseHistoryRows(level, rows) {
  if (level === "assets") {
    return rows
      .map((r) => ({ ts: String(r.timestamp || ""), symbol: r.symbol, value: toNumber(r.value) }))
      .filter((x) => x.ts && x.symbol);
  }
  if (level === "barca" || level === "groups") {
    return rows
      .map((r) => ({
        ts: String(r.timestamp || ""),
        key: level === "barca" ? r.barca : r.group || r.group_name,
        value: toNumber(r.value),
      }))
      .filter((x) => x.ts && x.key);
  }
  return rows
    .map((r) => ({ ts: String(r.timestamp || ""), value: toNumber(r.total_value) }))
    .filter((x) => x.ts);
}

// This app always displays timestamps in BRT (UTC-3).
export function toBrt(ts) {
  return dayjs(String(ts)).subtract(3, "hour");
}

// Bucket key for a BRT-shifted timestamp, one string per granularity so rows
// sharing a period collapse together.
export function bucketKeyFor(brtDate, granularity) {
  switch (granularity) {
    case "5min": {
      const minuteBucket = Math.floor(brtDate.minute() / 5) * 5;
      return `${brtDate.format("YYYY-MM-DD HH")}:${String(minuteBucket).padStart(2, "0")}`;
    }
    case "30min": {
      const minuteBucket = Math.floor(brtDate.minute() / 30) * 30;
      return `${brtDate.format("YYYY-MM-DD HH")}:${String(minuteBucket).padStart(2, "0")}`;
    }
    case "1h":
      return `${brtDate.format("YYYY-MM-DD HH")}:00`;
    case "4h": {
      const hourBucket = Math.floor(brtDate.hour() / 4) * 4;
      return `${brtDate.format("YYYY-MM-DD")} ${String(hourBucket).padStart(2, "0")}:00`;
    }
    case "weekly":
      return brtDate.startOf("week").format("YYYY-MM-DD");
    case "monthly":
      return brtDate.format("YYYY-MM");
    case "quarterly": {
      const quarter = Math.floor(brtDate.month() / 3) + 1;
      return `${brtDate.year()}-Q${quarter}`;
    }
    case "yearly":
      return `${brtDate.year()}`;
    case "daily":
    default:
      return brtDate.format("YYYY-MM-DD");
  }
}

// Group parsed rows into period buckets. For "assets"/"barca"/"groups" each
// bucket keeps the latest value per series (symbol/barca/group name) seen in
// that period; for "totals" each bucket keeps the single latest snapshot.
// `series` is empty for "totals".
export function groupHistoryByPeriod(level, parsedRows, granularity) {
  const groups = {};
  const isMultiSeries = level === "assets" || level === "barca" || level === "groups";

  parsedRows.forEach((item) => {
    const brt = toBrt(item.ts);
    if (!brt.isValid()) return;
    const key = bucketKeyFor(brt, granularity);

    if (isMultiSeries) {
      const seriesName = level === "assets" ? item.symbol : item.key;
      groups[key] = groups[key] || { ts: null };
      const existing = groups[key][seriesName];
      if (!existing || dayjs(item.ts).isAfter(dayjs(existing.ts))) {
        groups[key][seriesName] = { ts: item.ts, value: item.value };
      }
      if (!groups[key].ts || dayjs(item.ts).isAfter(dayjs(groups[key].ts))) {
        groups[key].ts = item.ts;
      }
    } else if (!groups[key] || dayjs(item.ts).isAfter(dayjs(groups[key].ts))) {
      groups[key] = { ts: item.ts, value: item.value };
    }
  });

  if (!isMultiSeries) {
    const data = Object.keys(groups)
      .map((k) => ({ period: k, value: groups[k].value, ts: groups[k].ts }))
      .sort((a, b) => (dayjs(a.ts).isBefore(dayjs(b.ts)) ? -1 : 1));
    return { data, series: [] };
  }

  const seriesSet = new Set();
  Object.values(groups).forEach((bucket) => {
    Object.keys(bucket).forEach((k) => {
      if (k !== "ts") seriesSet.add(k);
    });
  });
  const series = Array.from(seriesSet);

  const data = Object.keys(groups)
    .map((k) => {
      const entry = { period: k, ts: String(groups[k].ts) };
      series.forEach((s) => {
        const cell = groups[k][s];
        entry[s] = cell ? cell.value || 0 : 0;
      });
      return entry;
    })
    .sort((a, b) => (dayjs(a.ts).isBefore(dayjs(b.ts)) ? -1 : 1));

  return { data, series };
}

// Past a handful of series, a line chart stops being readable (color can't
// carry that much identity) — default to the top N by total value across the
// bucketed period, rather than dumping every series onto one chart.
export function topSeriesByValue(data, series, limit = 5) {
  const totals = series.map((key) => ({
    key,
    total: data.reduce((sum, row) => sum + (typeof row[key] === "number" ? row[key] : 0), 0),
  }));
  return totals
    .sort((a, b) => b.total - a.total)
    .slice(0, limit)
    .map((t) => t.key);
}
