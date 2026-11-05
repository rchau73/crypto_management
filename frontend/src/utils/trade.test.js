import { describe, expect, it } from "vitest";
import {
  assetSources,
  buildTradeRequest,
  cashSources,
  defaultCashSource,
  inputNumber,
  parsePositive,
  previewTrade,
  priceOf,
  tradablePositions,
} from "./trade";

const sources = [
  { symbol: "SYRUP", group_name: "DeFi", barca: "Altcoins", asset_class: "crypto", current_quantity: 10, notes: null },
  { symbol: "SYRUP", group_name: "DeFi", barca: "Altcoins", asset_class: "crypto", current_quantity: 40, notes: "Binance" },
  { symbol: "USDT", group_name: "Stable", barca: "Caixa", asset_class: "crypto", current_quantity: 500, notes: "Bybit" },
  { symbol: "USDC", group_name: "Stable", barca: "Caixa", asset_class: "crypto", current_quantity: 900, notes: "Ledger" },
  { symbol: "USDT", group_name: "Stable", barca: "Caixa", asset_class: "crypto", current_quantity: 100, notes: "binance " },
];

describe("sources", () => {
  it("lists a position's exchanges inside its BARCA, biggest first", () => {
    expect(assetSources(sources, "SYRUP", "Altcoins").map((s) => s.notes)).toEqual(["Binance", null]);
    expect(assetSources(sources, "SYRUP", "Base")).toEqual([]);
  });

  it("pays from the cash on the same exchange, else the biggest cash source", () => {
    const cash = cashSources(sources);
    expect(cash.map((c) => c.symbol + "@" + c.notes)).toEqual(["USDC@Ledger", "USDT@Bybit", "USDT@binance "]);
    expect(defaultCashSource(cash, "Binance").notes).toBe("binance ");
    expect(defaultCashSource(cash, "Kraken").symbol).toBe("USDC");
    expect(defaultCashSource([], "Binance")).toBeNull();
  });

  it("offers every non-cash position once and finds live prices", () => {
    const allocations = [
      { symbol: "BTC", barca: "Base", price: 80000 },
      { symbol: "BTC", barca: "Base", price: 80000 },
      { symbol: "USDT", barca: "Caixa", price: 1 },
      { symbol: "ADA", barca: "Altcoins", price: 0 },
    ];
    expect(tradablePositions(allocations)).toEqual([
      { symbol: "ADA", barca: "Altcoins" },
      { symbol: "BTC", barca: "Base" },
    ]);
    expect(priceOf(allocations, "BTC")).toBe(80000);
    expect(priceOf(allocations, "ADA")).toBeNull();
  });
});

describe("numbers", () => {
  it("accepts positive numbers, with a comma too", () => {
    expect(parsePositive("1.5")).toBe(1.5);
    expect(parsePositive(" 0,25 ")).toBe(0.25);
    for (const bad of ["", "0", "-1", "abc", undefined]) expect(parsePositive(bad)).toBeNull();
  });

  it("pre-fills inputs with at most 8 decimals", () => {
    expect(inputNumber(4914.17 / 80300)).toBe("0.06119763");
    expect(inputNumber(0)).toBe("");
  });
});

describe("previewTrade", () => {
  it("a buy adds to the exchange and spends the cash", () => {
    const preview = previewTrade({ side: "buy", held: 40, quantity: 10, price: 0.5, useCash: true, cashHeld: 100, cashQuantity: 5 });
    expect(preview).toEqual({ total: 5, newHeld: 50, newCash: 95, problems: [] });
  });

  it("a sell can't take more than the exchange or leave the cash negative", () => {
    const preview = previewTrade({ side: "sell", held: 4, quantity: 10, price: 1, useCash: false, cashHeld: 0, cashQuantity: null });
    expect(preview.problems).toEqual(["That exchange holds only 4; you can't sell more."]);
    const overspend = previewTrade({ side: "buy", held: 0, quantity: 1, price: 1, useCash: true, cashHeld: 3, cashQuantity: 5 });
    expect(overspend.problems).toEqual(["The cash source holds only 3."]);
  });

  it("asks for the missing numbers", () => {
    const preview = previewTrade({ side: "buy", held: 0, quantity: null, price: null, useCash: true, cashHeld: 0, cashQuantity: null });
    expect(preview.problems).toEqual(["Enter a quantity above 0.", "Enter a price above 0."]);
    const noCash = previewTrade({ side: "buy", held: 0, quantity: 1, price: 1, useCash: true, cashHeld: 0, cashQuantity: null });
    expect(noCash.problems).toEqual(["Enter the cash moved, above 0."]);
    expect(preview.total).toBeNull();
  });
});

describe("buildTradeRequest", () => {
  it("names the new exchange and the cash source", () => {
    const [syrup] = assetSources(sources, "SYRUP", "Altcoins");
    const [usdc] = cashSources(sources);
    expect(
      buildTradeRequest({ side: "buy", asset: syrup, exchange: " Kraken ", quantity: 2, price: 0.5, cash: usdc, cashQuantity: 1 })
    ).toEqual({
      side: "buy",
      asset: { symbol: "SYRUP", group_name: "DeFi", barca: "Altcoins", asset_class: "crypto", source: "Kraken" },
      quantity: 2,
      price: 0.5,
      cash: { position: { symbol: "USDC", group_name: "Stable", barca: "Caixa", asset_class: "crypto", source: "Ledger" }, quantity: 1 },
    });
  });

  it("sends no cash leg when the checkbox is off, and null for the unnamed source", () => {
    const unnamed = assetSources(sources, "SYRUP", "Altcoins")[1];
    const body = buildTradeRequest({ side: "sell", asset: unnamed, exchange: unnamed.notes, quantity: 1, price: 1, cash: null });
    expect(body.asset.source).toBeNull();
    expect(body.cash).toBeNull();
  });
});
