import { describe, expect, it } from "vitest";
import { buildSearchIndex, searchIndex } from "./search";

const pages = [
  { key: "holdings", label: "Holdings" },
  { key: "history", label: "History" },
];
const allocations = [
  { symbol: "ETH", group: "Core", barca: "Altcoins" },
  { symbol: "ETH", group: "Staking", barca: "RendaPassiva" },
  { symbol: "WETH", group: "DeFi", barca: "Altcoins" },
  { symbol: "BTC", group: "Core", barca: "Base" },
];
const index = buildSearchIndex(pages, allocations);
const labels = (results) => results.map((r) => r.label);

describe("searchIndex", () => {
  it("finds an asset and leads to its row in the Asset view", () => {
    const [eth] = searchIndex(index, "eth");
    expect(eth).toMatchObject({ label: "ETH", kind: "Asset · Altcoins, RendaPassiva", page: "holdings", params: { view: "asset", q: "ETH" } });
  });

  it("puts names that start with the query first", () => {
    expect(labels(searchIndex(index, "eth"))).toEqual(["ETH", "WETH"]);
  });

  it("finds pages, BARCAs and groups too", () => {
    expect(searchIndex(index, "hist")[0]).toMatchObject({ kind: "Page", page: "history" });
    expect(searchIndex(index, "base")[0]).toMatchObject({ kind: "BARCA", params: { view: "barca", q: "Base" } });
    expect(searchIndex(index, "staking")[0]).toMatchObject({ kind: "Group", params: { view: "group", q: "Staking" } });
  });

  it("lists each asset, BARCA and group once", () => {
    expect(labels(searchIndex(index, "core"))).toEqual(["Core"]);
  });

  it("returns nothing for an empty query, and caps long result lists", () => {
    expect(searchIndex(index, "  ")).toEqual([]);
    const many = buildSearchIndex([], Array.from({ length: 20 }, (_, i) => ({ symbol: `T${i}` })));
    expect(searchIndex(many, "t")).toHaveLength(8);
  });
});
