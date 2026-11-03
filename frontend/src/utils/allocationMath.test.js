import { describe, expect, it } from "vitest";
import {
  aggregateBySymbol,
  computeYAxisDomain,
  deriveRowMetrics,
  bandSeverity,
  getTotalTargetPercent,
  valueGapFromDeviation,
} from "./allocationMath";

describe("deriveRowMetrics", () => {
  it("computes current %, deviation, value deviation and DCA against a total", () => {
    const row = { value: 500, target_percent: 40 };
    const metrics = deriveRowMetrics(row, 1000);
    expect(metrics.currentPercent).toBe(50);
    expect(metrics.deviation).toBe(10); // 50% actual vs 40% target
    expect(metrics.valueDeviation).toBe(100); // 500 - (1000 * 0.4)
    expect(metrics.dca).toBeCloseTo(30); // 30% of |100|
  });

  it("does not divide by zero when the total is zero", () => {
    const metrics = deriveRowMetrics({ value: 0, target_percent: 10 }, 0);
    expect(metrics.currentPercent).toBe(0);
    expect(Number.isFinite(metrics.deviation)).toBe(true);
  });

  it("treats a missing target_percent as 0, not NaN", () => {
    const metrics = deriveRowMetrics({ value: 100 }, 1000);
    expect(metrics.deviation).toBe(10);
    expect(Number.isNaN(metrics.valueDeviation)).toBe(false);
  });
});

describe("bandSeverity (BARCA rule 1: target ± 20% of target)", () => {
  it("is high outside the band, whatever the target size", () => {
    expect(bandSeverity(38, 50)).toBe("high"); // band 40-60
    expect(bandSeverity(13, 10)).toBe("high"); // band 8-12
    expect(bandSeverity(0, 5)).toBe("high"); // fully sold, still targeted
  });

  it("is normal inside the band, even when several points off a big target", () => {
    // Regression: a 3-point miss on a 50% target used to be red (">1 point").
    expect(bandSeverity(53, 50)).toBe("normal");
    expect(bandSeverity(11.5, 10)).toBe("normal");
  });

  it("treats the band edge as inside ('more than 20%')", () => {
    expect(bandSeverity(60, 50)).toBe("normal");
    expect(bandSeverity(40, 50)).toBe("normal");
  });

  it("flags holdings with no target, but not an empty untargeted row", () => {
    expect(bandSeverity(0.5, 0)).toBe("high");
    expect(bandSeverity(0, 0)).toBe("normal");
    expect(bandSeverity(undefined, undefined)).toBe("normal");
  });
});

describe("valueGapFromDeviation", () => {
  it("measures the points against the whole wallet, not the row's value", () => {
    // A group worth $2,000 that is 5 points over target in a $10,000 wallet
    // is $500 over. Regression: it used to show 5% of $2,000 = $100.
    expect(valueGapFromDeviation(5, 10000)).toBe(500);
    expect(valueGapFromDeviation(-2.5, 10000)).toBe(-250);
  });

  it("is 0 for a missing deviation or an empty wallet", () => {
    expect(valueGapFromDeviation(undefined, 10000)).toBe(0);
    expect(valueGapFromDeviation(5, 0)).toBe(0);
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
