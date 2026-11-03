import { describe, expect, it } from "vitest";
import {
  aggregateBySymbol,
  computeYAxisDomain,
  bandStatus,
  getTotalTargetPercent,
} from "./allocationMath";

describe("bandStatus (BARCA rule 1: target ± 20% of target)", () => {
  it("is above or below outside the band, whatever the target size", () => {
    expect(bandStatus(38, 50)).toBe("below"); // band 40-60
    expect(bandStatus(13, 10)).toBe("above"); // band 8-12
    expect(bandStatus(0, 5)).toBe("below"); // fully sold, still targeted
  });

  it("is in band even when several points off a big target", () => {
    // Regression: a 3-point miss on a 50% target used to be red (">1 point").
    expect(bandStatus(53, 50)).toBe("in");
    expect(bandStatus(11.5, 10)).toBe("in");
  });

  it("treats the band edge as inside ('more than 20%')", () => {
    expect(bandStatus(60, 50)).toBe("in");
    expect(bandStatus(40, 50)).toBe("in");
  });

  it("puts holdings with no target above the band, but not an empty untargeted row", () => {
    expect(bandStatus(0.5, 0)).toBe("above");
    expect(bandStatus(0, 0)).toBe("in");
    expect(bandStatus(undefined, undefined)).toBe("in");
  });
});

describe("getTotalTargetPercent", () => {
  it("sums target_percent across rows, ignoring non-numeric values", () => {
    expect(getTotalTargetPercent([{ target_percent: 10 }, { target_percent: 20 }])).toBe(30);
  });

  it("returns 0 for an empty list", () => {
    expect(getTotalTargetPercent([])).toBe(0);
  });

  it("ignores rows with a missing/invalid target_percent", () => {
    expect(getTotalTargetPercent([{ target_percent: 10 }, { target_percent: undefined }, {}])).toBe(10);
  });
});

describe("aggregateBySymbol", () => {
  it("sums quantity/value/target across rows sharing a symbol", () => {
    const rows = [
      { symbol: "BTC", current_quantity: 1, value: 100, target_percent: 30, price: 100 },
      { symbol: "BTC", current_quantity: 0.5, value: 50, target_percent: 10, price: 100 },
      { symbol: "ETH", current_quantity: 2, value: 40, target_percent: 5, price: 20 },
    ];
    const result = aggregateBySymbol(rows);
    const btc = result.find((r) => r.symbol === "BTC");
    expect(btc.current_quantity).toBe(1.5);
    expect(btc.value).toBe(150);
    expect(btc.target_percent).toBe(40);
    expect(result).toHaveLength(2);
  });

  it("skips rows without a symbol", () => {
    expect(aggregateBySymbol([{ value: 100 }])).toEqual([]);
  });

  it("derives price from value/quantity, falling back to last known price when quantity is 0", () => {
    const result = aggregateBySymbol([{ symbol: "BTC", current_quantity: 0, value: 0, price: 42 }]);
    expect(result[0].price).toBe(42);
  });
});

describe("computeYAxisDomain", () => {
  it("pads the min/max of the given keys across rows", () => {
    const rows = [{ a: 10 }, { a: 20 }];
    const [min, max] = computeYAxisDomain(rows, ["a"]);
    expect(min).toBeLessThan(10);
    expect(max).toBeGreaterThan(20);
  });

  it("falls back to dataMin/dataMax for empty input", () => {
    expect(computeYAxisDomain([], ["a"])).toEqual(["dataMin", "dataMax"]);
    expect(computeYAxisDomain([{ a: 1 }], [])).toEqual(["dataMin", "dataMax"]);
  });

  it("pads a flat (all-equal) series so it isn't a zero-height line", () => {
    const [min, max] = computeYAxisDomain([{ a: 5 }, { a: 5 }], ["a"]);
    expect(min).toBeLessThan(5);
    expect(max).toBeGreaterThan(5);
  });
});
