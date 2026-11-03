import { describe, expect, it } from "vitest";
import {
  barcaNames,
  barcaShift,
  buildTransferRequest,
  defaultGroup,
  destination,
  exchangeNames,
  isSamePlace,
  movableSymbols,
  originOptions,
  previewMove,
} from "./move";

const btc = (barca, group, notes, qty) => ({ symbol: "BTC", group_name: group, barca, asset_class: "crypto", current_quantity: qty, notes });
const sources = [
  btc("Base", "Base", "Binance", 0.5),
  btc("Base", "Base", "Ledger", 1),
  btc("Base", "Base", "Old", 0),
  { symbol: "MSTY", group_name: "Yield", barca: "RendaPassiva", asset_class: "us-indices", current_quantity: 3, notes: "Avenue" },
];

describe("pickers", () => {
  it("offers only what is held, biggest origin first", () => {
    expect(movableSymbols(sources)).toEqual(["BTC", "MSTY"]);
    expect(originOptions(sources, "BTC").map((o) => o.notes)).toEqual(["Ledger", "Binance"]);
  });

  it("knows the BARCAs, their groups and the exchanges", () => {
    const report = { barcaAllocations: [{ barca: "Base" }, { barca: "Caixa" }], barcaActualAllocations: [{ barca: "Base" }, { barca: "AÇÕES" }] };
    expect(barcaNames(report)).toEqual(["AÇÕES", "Base", "Caixa"]);
    expect(defaultGroup(sources, "BTC", "Base")).toBe("Base");
    expect(defaultGroup(sources, "BTC", "RendaPassiva")).toBe("Yield");
    expect(defaultGroup(sources, "BTC", "Caixa")).toBe("Caixa");
    expect(exchangeNames(sources)).toEqual(["Avenue", "Binance", "Ledger", "Old"]);
  });

  it("finds the destination's quantity and whether its position is new", () => {
    expect(destination(sources, { symbol: "BTC", assetClass: "crypto", group: "Base", barca: "Base", exchange: "Binance" })).toEqual({
      held: 0.5,
      newPosition: false,
    });
    expect(destination(sources, { symbol: "BTC", assetClass: "crypto", group: "Yield", barca: "RendaPassiva", exchange: "Deribit" })).toEqual({
      held: 0,
      newPosition: true,
    });
    expect(isSamePlace(sources[0], { group: "Base ", barca: "Base", exchange: " Binance" })).toBe(true);
  });
});

describe("previewMove", () => {
  const base = { held: 0.5, destHeld: 0, samePlace: false, group: "Base", barca: "Base" };

  it("moves the quantity, less the fee", () => {
    const preview = previewMove({ ...base, quantity: 0.5, received: 0.4995 });
    expect(preview).toMatchObject({ fromAfter: 0, toAfter: 0.4995, problems: [] });
    expect(preview.fee).toBeCloseTo(0.0005, 10);
  });

  it("refuses an overdraw, a received above sent, a missing destination and the same place", () => {
    expect(previewMove({ ...base, quantity: 0.6, received: 0.6 }).problems).toEqual(["The origin holds only 0.5."]);
    expect(previewMove({ ...base, quantity: 0.1, received: 0.2 }).problems).toEqual(["You can't receive more than you sent."]);
    expect(previewMove({ ...base, quantity: 0.1, received: 0.1, barca: "", group: " " }).problems).toEqual([
      "Pick the destination BARCA.",
      "Name the destination group.",
    ]);
    expect(previewMove({ ...base, quantity: 0.1, received: 0.1, samePlace: true }).problems).toEqual(["The destination is the same as the origin."]);
  });
});

describe("barcaShift", () => {
  const report = { barcaActualAllocations: [{ barca: "Base", value: 600 }, { barca: "RendaPassiva", value: 100 }, { barca: "Caixa", value: 300 }] };

  it("shows both BARCAs before and after", () => {
    expect(barcaShift(report, { fromBarca: "Base", toBarca: "RendaPassiva", sentValue: 100, receivedValue: 100 })).toEqual([
      { barca: "Base", before: 60, after: 50 },
      { barca: "RendaPassiva", before: 10, after: 20 },
    ]);
  });

  it("says nothing for a fee-less move inside one BARCA", () => {
    expect(barcaShift(report, { fromBarca: "Base", toBarca: "Base", sentValue: 100, receivedValue: 100 })).toEqual([]);
  });
});

describe("buildTransferRequest", () => {
  it("keeps the origin's key and sends received only when a fee was taken", () => {
    const [ledger] = originOptions(sources, "BTC");
    expect(buildTransferRequest({ origin: ledger, group: " Yield ", barca: "RendaPassiva", exchange: "Deribit", quantity: 0.15, received: 0.15 })).toEqual({
      from: { symbol: "BTC", group_name: "Base", barca: "Base", asset_class: "crypto", source: "Ledger" },
      to: { symbol: "BTC", group_name: "Yield", barca: "RendaPassiva", asset_class: "crypto", source: "Deribit" },
      quantity: 0.15,
      received: null,
    });
    expect(buildTransferRequest({ origin: ledger, group: "Base", barca: "Base", exchange: "", quantity: 1, received: 0.9 }).to.source).toBeNull();
  });
});
