import { describe, expect, it } from "vitest";
import { matchUnpriced, unpricedFixes, unpricedTitle } from "./unpriced";

const fund = { symbol: "FUND A", group: "Holding", barca: "RendaFixa", asset_class: "br-equities", current_quantity: 1000 };
const bond = { symbol: "BOND B", group: "Holding", barca: "RendaFixa", asset_class: "br-equities", current_quantity: 40 };
const cash = { symbol: "CASH BRL", group: "Holding", barca: "Caixa", asset_class: "brl-manual", current_quantity: 500 };

describe("unpriced", () => {
  it("titles one or many holdings", () => {
    expect(unpricedTitle(1)).toBe("1 holding has no price and is left out of every total");
    expect(unpricedTitle(3)).toBe("3 holdings have no price and are left out of every total");
  });

  it("gives one fix per asset class, pointing B3 misses at brl-manual", () => {
    const fixes = unpricedFixes([fund, bond, cash]);
    expect(fixes).toHaveLength(2);
    expect(fixes[0]).toMatch(/brl-manual/);
    expect(fixes[1]).toMatch(/USD\/BRL rate/);
  });

  it("falls back to a generic fix for any other class", () => {
    expect(unpricedFixes([{ ...fund, asset_class: "crypto" }])).toEqual([
      "Check the symbol in Portfolio Targets: the price provider doesn't know it.",
    ]);
  });

  it("matches the filter on name, group and BARCA, case-insensitively", () => {
    expect(matchUnpriced([fund, cash], "rendafixa")).toEqual([fund]);
    expect(matchUnpriced([fund, cash], "cash")).toEqual([cash]);
    expect(matchUnpriced([fund, cash], "  ")).toEqual([]);
    expect(matchUnpriced([], "xp")).toEqual([]);
  });
});
