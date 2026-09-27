import { describe, expect, it } from "vitest";
import { formatNumber, formatPrice, formatUsdAxisTick } from "./formatters";

describe("formatNumber", () => {
  it("formats with 2 decimal places and thousands separators", () => {
    expect(formatNumber(1234.5)).toBe("1,234.50");
  });

  it("returns a dash for non-numbers", () => {
    expect(formatNumber(undefined)).toBe("-");
    expect(formatNumber(NaN)).toBe("-");
    expect(formatNumber("12")).toBe("-");
  });

  it("handles zero and negative numbers", () => {
    expect(formatNumber(0)).toBe("0.00");
    expect(formatNumber(-5.1)).toBe("-5.10");
  });
});

describe("formatPrice", () => {
  it("formats with 4 decimal places", () => {
    expect(formatPrice(0.12345)).toBe("0.1235");
  });

  it("returns a dash for non-numbers", () => {
    expect(formatPrice(null)).toBe("-");
  });
});

describe("formatUsdAxisTick", () => {
  it("compacts large values with a currency symbol", () => {
    expect(formatUsdAxisTick(1500)).toBe("$2K");
  });

  it("returns an empty string for non-numbers", () => {
    expect(formatUsdAxisTick(undefined)).toBe("");
    expect(formatUsdAxisTick(NaN)).toBe("");
  });
});
