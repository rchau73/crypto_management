import { describe, expect, it } from "vitest";

import { formatNumber, sumTargetPercent } from "./format";

describe("formatNumber", () => {
  it("formats numeric values with two decimal places", () => {
    expect(formatNumber(1234.5)).toBe("1,234.50");
  });

  it("returns dash for non-numeric values", () => {
    expect(formatNumber("abc")).toBe("-");
    expect(formatNumber(Number.NaN)).toBe("-");
  });
});

describe("sumTargetPercent", () => {
  it("adds numeric target percentages", () => {
    const rows = [{ target_percent: 10 }, { target_percent: 20 }];
    expect(sumTargetPercent(rows)).toBeCloseTo(30);
  });

  it("ignores non-numeric entries", () => {
    const rows = [
      { target_percent: "15" },
      { target_percent: "invalid" },
      { target_percent: 5 },
    ];
    expect(sumTargetPercent(rows)).toBeCloseTo(20);
  });
});
