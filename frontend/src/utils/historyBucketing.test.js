import { describe, expect, it } from "vitest";
import { bucketKeyFor, groupHistoryByPeriod, parseHistoryRows, toBrt, topSeriesByValue } from "./historyBucketing";

describe("parseHistoryRows", () => {
  it("extracts symbol/value pairs for the assets level", () => {
    const rows = [{ timestamp: "2024-01-01T00:00:00Z", symbol: "BTC", value: 100 }];
    expect(parseHistoryRows("assets", rows)).toEqual([{ ts: "2024-01-01T00:00:00Z", symbol: "BTC", value: 100 }]);
  });

  it("drops rows missing a timestamp or symbol", () => {
    const rows = [{ timestamp: "", symbol: "BTC", value: 1 }, { timestamp: "t", value: 1 }];
    expect(parseHistoryRows("assets", rows)).toEqual([]);
  });

  it("reads barca or group_name depending on level for barca/groups", () => {
    const barcaRows = [{ timestamp: "t", barca: "Base", value: 10 }];
    expect(parseHistoryRows("barca", barcaRows)).toEqual([{ ts: "t", key: "Base", value: 10 }]);

    const groupRows = [{ timestamp: "t", group_name: "Core", value: 10 }];
    expect(parseHistoryRows("groups", groupRows)).toEqual([{ ts: "t", key: "Core", value: 10 }]);
  });

  it("reads total_value for the totals level and coerces non-numeric values to 0", () => {
    const rows = [{ timestamp: "t", total_value: "not-a-number" }];
    expect(parseHistoryRows("totals", rows)).toEqual([{ ts: "t", value: 0 }]);
  });
});

describe("bucketKeyFor", () => {
  const d = toBrt("2024-03-15T14:37:00Z"); // 11:37 BRT

  it("buckets to the hour for 1h granularity", () => {
    expect(bucketKeyFor(d, "1h")).toBe(`${d.format("YYYY-MM-DD HH")}:00`);
  });

  it("buckets to the day for daily granularity (and as the default)", () => {
    expect(bucketKeyFor(d, "daily")).toBe(d.format("YYYY-MM-DD"));
    expect(bucketKeyFor(d, "unknown-granularity")).toBe(d.format("YYYY-MM-DD"));
  });

  it("buckets to the year for yearly granularity", () => {
    expect(bucketKeyFor(d, "yearly")).toBe(String(d.year()));
  });
});

describe("groupHistoryByPeriod", () => {
  it("keeps the latest snapshot per period for totals", () => {
    const parsed = [
      { ts: "2024-01-01T00:00:00Z", value: 100 },
      { ts: "2024-01-01T01:00:00Z", value: 150 },
    ];
    const { data, series } = groupHistoryByPeriod("totals", parsed, "daily");
    expect(series).toEqual([]);
    expect(data).toHaveLength(1);
    expect(data[0].value).toBe(150); // the later of the two same-day snapshots wins
  });

  it("keeps the latest value per series per period for assets, and detects the series list", () => {
    const parsed = [
      { ts: "2024-01-01T00:00:00Z", symbol: "BTC", value: 100 },
      { ts: "2024-01-01T01:00:00Z", symbol: "BTC", value: 110 },
      { ts: "2024-01-01T00:30:00Z", symbol: "ETH", value: 50 },
    ];
    const { data, series } = groupHistoryByPeriod("assets", parsed, "daily");
    expect(series.sort()).toEqual(["BTC", "ETH"]);
    expect(data).toHaveLength(1);
    expect(data[0].BTC).toBe(110);
    expect(data[0].ETH).toBe(50);
  });

  it("skips rows with an unparsable timestamp instead of throwing", () => {
    const parsed = [{ ts: "not-a-date", value: 1 }];
    expect(groupHistoryByPeriod("totals", parsed, "daily").data).toEqual([]);
  });

  it("returns periods sorted chronologically", () => {
    const parsed = [
      { ts: "2024-02-01T00:00:00Z", value: 2 },
      { ts: "2024-01-01T00:00:00Z", value: 1 },
    ];
    const { data } = groupHistoryByPeriod("totals", parsed, "monthly");
    expect(data.map((d) => d.value)).toEqual([1, 2]);
  });
});

describe("topSeriesByValue", () => {
  it("picks the N series with the highest total value", () => {
    const data = [
      { period: "d1", BTC: 100, ETH: 10, SOL: 5 },
      { period: "d2", BTC: 100, ETH: 10, SOL: 5 },
    ];
    expect(topSeriesByValue(data, ["BTC", "ETH", "SOL"], 2)).toEqual(["BTC", "ETH"]);
  });

  it("returns every series when there are fewer than the limit", () => {
    const data = [{ period: "d1", BTC: 1, ETH: 2 }];
    expect(topSeriesByValue(data, ["BTC", "ETH"], 5)).toHaveLength(2);
  });

  it("treats a missing/non-numeric cell as 0 rather than throwing", () => {
    const data = [{ period: "d1", BTC: 100 }]; // ETH absent from this bucket
    expect(topSeriesByValue(data, ["BTC", "ETH"], 1)).toEqual(["BTC"]);
  });
});
