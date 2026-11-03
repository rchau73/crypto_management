import { describe, expect, it } from "vitest";
import { buildHoldingRows, filterHoldingRows } from "./holdings";

// A $1,000 wallet: BTC in two groups, ETH, and USDT with no target.
const report = {
  allocations: [
    { symbol: "BTC", group: "Core", barca: "Base", price: 100, current_quantity: 3, value: 300, target_percent: 30 },
    { symbol: "BTC", group: "Trading", barca: "Base", price: 100, current_quantity: 1, value: 100, target_percent: 20 },
    { symbol: "ETH", group: "Core", barca: "Altcoins", price: 10, current_quantity: 50, value: 500, target_percent: 30 },
    { symbol: "USDT", group: "Cash", barca: "Caixa", price: 1, current_quantity: 100, value: 100, target_percent: 0 },
  ],
  groupAllocations: [
    { group: "Core", value: 800, target_percent: 60 },
    { group: "Trading", value: 100, target_percent: 20 },
    { group: "Cash", value: 100, target_percent: 0 },
  ],
  barcaAllocations: [
    { barca: "Base", value: 400, target_percent: 50 },
    { barca: "Altcoins", value: 500, target_percent: 30 },
    { barca: "Caixa", value: 100, target_percent: 20 },
  ],
  barcaActualAllocations: [
    { barca: "Base", value: 400 },
    { barca: "Altcoins", value: 500 },
    { barca: "Caixa", value: 100 },
  ],
};

const byName = (rows) => Object.fromEntries(rows.map((r) => [r.name, r]));

describe("buildHoldingRows", () => {
  it("asset view: one row per symbol, measured against the whole wallet", () => {
    const { rows, totalValue } = buildHoldingRows("asset", report);
    const btc = byName(rows).BTC;
    expect(totalValue).toBe(1000);
    expect(rows).toHaveLength(3);
    expect(btc.value).toBe(400);
    expect(btc.targetPercent).toBe(50);
    expect(btc.currentPercent).toBe(40);
    expect(btc.group).toBe("Core, Trading");
    expect(btc.gap).toBe(-100); // $400 held vs $500 target
    expect(btc.status).toBe("in"); // 40% is inside 40-60
  });

  it("gives the DCA step a direction: sell above target, buy below", () => {
    const rows = byName(buildHoldingRows("asset", report).rows);
    expect(rows.ETH.step).toEqual({ verb: "Sell", amount: 60 }); // 30% of +$200
    expect(rows.BTC.step).toEqual({ verb: "Buy", amount: 30 }); // 30% of -$100
    expect(rows.ETH.status).toBe("above"); // 50% vs band 24-36
  });

  it("flags value held with no target, and has no step for a row on target", () => {
    const rows = byName(buildHoldingRows("asset", report).rows);
    expect(rows.USDT.status).toBe("above");
    const onTarget = buildHoldingRows("asset", {
      allocations: [{ symbol: "X", value: 100, target_percent: 100 }],
    }).rows[0];
    expect(onTarget.step).toBeNull();
  });

  it("position view keeps each group/BARCA row apart", () => {
    const { rows } = buildHoldingRows("position", report);
    expect(rows).toHaveLength(4);
    expect(rows[1]).toMatchObject({ name: "BTC", group: "Trading", quantity: 1, currentPercent: 10 });
  });

  it("group and BARCA views come from the server's totals", () => {
    expect(byName(buildHoldingRows("group", report).rows).Core.deviation).toBe(20);
    const barca = byName(buildHoldingRows("barca", report).rows);
    expect(barca.Caixa).toMatchObject({ currentPercent: 10, status: "below" }); // band 16-24
  });

  it("BARCA view also lists value held in a BARCA with no target", () => {
    const { rows } = buildHoldingRows("barca", {
      ...report,
      barcaActualAllocations: [...report.barcaActualAllocations, { barca: "Untargeted", value: 50 }],
    });
    expect(byName(rows).Untargeted).toMatchObject({ targetPercent: 0, status: "above" });
  });

  it("is empty, not NaN, for an empty account", () => {
    const { rows, totalValue } = buildHoldingRows("asset", { allocations: [] });
    expect(rows).toEqual([]);
    expect(totalValue).toBe(0);
    expect(buildHoldingRows("barca", {}).rows).toEqual([]);
  });
});

describe("filterHoldingRows", () => {
  const { rows } = buildHoldingRows("asset", report);

  it("matches the name, group or BARCA, ignoring case", () => {
    expect(filterHoldingRows(rows, { query: "eth" }).map((r) => r.name)).toEqual(["ETH"]);
    expect(filterHoldingRows(rows, { query: "trading" }).map((r) => r.name)).toEqual(["BTC"]);
    expect(filterHoldingRows(rows, { query: "caixa" }).map((r) => r.name)).toEqual(["USDT"]);
  });

  it("can keep only rows outside their band", () => {
    expect(filterHoldingRows(rows, { outOfBandOnly: true }).map((r) => r.name).sort()).toEqual(["ETH", "USDT"]);
  });

  it("returns every row with no filter", () => {
    expect(filterHoldingRows(rows)).toHaveLength(3);
  });
});
