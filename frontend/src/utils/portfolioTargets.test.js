import { describe, expect, it } from "vitest";
import {
  blankRow,
  canCorrectQuantity,
  filterRows,
  parseNonNegative,
  sortValue,
  toSavePayload,
  uniqueSorted,
  validateRows,
} from "./portfolioTargets";

const existing = (symbol, target, extra = {}) => ({
  symbol,
  group_name: "Core",
  barca: "Base",
  asset_class: "crypto",
  target_percent: target,
  current_quantity: 3,
  notes: "Binance | Ledger",
  source_count: 2,
  isNew: false,
  ...extra,
});

describe("parseNonNegative", () => {
  it("accepts zero and positive numbers, including numeric strings", () => {
    expect(parseNonNegative("0")).toBe(0);
    expect(parseNonNegative("1.5")).toBe(1.5);
    expect(parseNonNegative(42)).toBe(42);
  });

  it("rejects empty, negative and non-numeric input instead of turning it into 0", () => {
    for (const bad of ["", null, undefined, "-1", "abc", "1e999"]) {
      expect(parseNonNegative(bad)).toBeNull();
    }
  });
});

describe("canCorrectQuantity", () => {
  it("is only true for an existing single-source row", () => {
    expect(canCorrectQuantity(existing("BTC", 0, { source_count: 1 }))).toBe(true);
    expect(canCorrectQuantity(existing("BTC", 0, { source_count: 2 }))).toBe(false);
    expect(canCorrectQuantity({ ...blankRow(), source_count: 1 })).toBe(false);
  });

  it("does not rely on the notes text (a NULL-notes source is invisible there)", () => {
    expect(canCorrectQuantity(existing("BTC", 0, { notes: "Binance", source_count: 2 }))).toBe(false);
  });
});

describe("validateRows", () => {
  it("is valid when every row is valid and targets sum to 100", () => {
    expect(validateRows([existing("BTC", 60), existing("ETH", "40")])).toEqual({ sum: 100, isValid: true });
  });

  it("rejects a bad sum, an empty table, a blank symbol or an out-of-range target", () => {
    expect(validateRows([existing("BTC", 60)]).isValid).toBe(false);
    expect(validateRows([]).isValid).toBe(false);
    expect(validateRows([existing(" ", 100)]).isValid).toBe(false);
    expect(validateRows([existing("BTC", -50), existing("ETH", 150)]).isValid).toBe(false);
  });

  it("rejects a new row whose starting quantity isn't a number >= 0", () => {
    const newRow = { ...blankRow(), symbol: "HGRU11", target_percent: 100, current_quantity: "-3" };
    expect(validateRows([newRow]).isValid).toBe(false);
    expect(validateRows([{ ...newRow, current_quantity: "3" }]).isValid).toBe(true);
  });
});

describe("toSavePayload", () => {
  it("never sends quantity or notes for an existing row", () => {
    const [row] = toSavePayload([existing("BTC", "100")]);
    expect(row).toEqual({ symbol: "BTC", group_name: "Core", barca: "Base", asset_class: "crypto", target_percent: 100 });
  });

  it("sends a new row's trimmed starting quantity and notes", () => {
    const newRow = { ...blankRow(), symbol: " HGRU11 ", group_name: " ", target_percent: "100", current_quantity: "10", notes: " XP " };
    expect(toSavePayload([newRow])).toEqual([
      {
        symbol: "HGRU11",
        group_name: null,
        barca: null,
        asset_class: "crypto",
        target_percent: 100,
        current_quantity: 10,
        notes: "XP",
      },
    ]);
  });
});

describe("uniqueSorted", () => {
  it("dedupes, sorts and drops empty values", () => {
    expect(uniqueSorted(["b", "", "a", "b", null])).toEqual(["a", "b"]);
  });
});

describe("filterRows", () => {
  const rows = [
    { symbol: "BTC", group_name: "Holding", notes: "Binance | Ledger Wallet", current_quantity: 0.6 },
    { symbol: "ETH", group_name: "DeFi", notes: "Binance USDC", current_quantity: 2 },
    { symbol: "SOL", group_name: null, notes: null, current_quantity: 0 },
  ];
  const symbols = (result) => result.map((r) => r.symbol);

  it("returns every row when no filter is typed (blank text counts as none)", () => {
    expect(filterRows(rows, { symbol: "", notes: "   " })).toBe(rows);
  });

  it("matches text inside the column, ignoring case and surrounding spaces", () => {
    expect(symbols(filterRows(rows, { notes: " binance " }))).toEqual(["BTC", "ETH"]);
    expect(symbols(filterRows(rows, { notes: "ledger" }))).toEqual(["BTC"]);
  });

  it("requires every typed filter to match", () => {
    expect(symbols(filterRows(rows, { notes: "binance", group_name: "defi" }))).toEqual(["ETH"]);
  });

  it("treats null values as empty, not as the text 'null'", () => {
    expect(filterRows(rows, { group_name: "null" })).toEqual([]);
  });

  it("matches numbers by their text", () => {
    expect(symbols(filterRows(rows, { current_quantity: "0.6" }))).toEqual(["BTC"]);
  });

  it("always keeps a new, unsaved row", () => {
    const fresh = { ...blankRow(), symbol: "" };
    expect(filterRows([...rows, fresh], { symbol: "BTC" })).toEqual([rows[0], fresh]);
  });
});

describe("sortValue", () => {
  it("sorts Target % and Quantity as numbers, even when typed as text", () => {
    expect(sortValue({ target_percent: "12.5" }, "target_percent")).toBe(12.5);
    expect(sortValue({ current_quantity: "" }, "current_quantity")).toBe(0);
  });

  it("turns a missing text value into an empty string", () => {
    expect(sortValue({ notes: null }, "notes")).toBe("");
    expect(sortValue({ symbol: "BTC" }, "symbol")).toBe("BTC");
  });
});
