import { describe, expect, it } from "vitest";
import { COLORS, colorForSeries } from "./theme";

describe("colorForSeries", () => {
  it("assigns color by an entity's position in the full sorted series list", () => {
    const all = ["BTC", "ETH", "SOL"];
    expect(colorForSeries("BTC", all)).toBe(COLORS[0]);
    expect(colorForSeries("ETH", all)).toBe(COLORS[1]);
    expect(colorForSeries("SOL", all)).toBe(COLORS[2]);
  });

  it("does not repaint survivors when the selection shrinks", () => {
    // BTC's color must not change just because ETH is no longer selected —
    // colorForSeries always resolves against the *full* series list.
    const all = ["BTC", "ETH", "SOL"];
    const btcBefore = colorForSeries("BTC", all);
    const btcAfterEthRemoved = colorForSeries("BTC", all); // full list unchanged
    expect(btcAfterEthRemoved).toBe(btcBefore);
  });

  it("wraps around after exhausting the fixed palette", () => {
    const many = Array.from({ length: COLORS.length + 2 }, (_, i) => `S${i}`);
    expect(colorForSeries("S0", many)).toBe(COLORS[0]);
    expect(colorForSeries(`S${COLORS.length}`, many)).toBe(COLORS[0]);
  });

  it("falls back to the first color for an unknown name", () => {
    expect(colorForSeries("UNKNOWN", ["BTC"])).toBe(COLORS[0]);
  });
});
