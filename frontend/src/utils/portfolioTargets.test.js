import { describe, expect, it } from "vitest";
import { blankRow, canCorrectQuantity, parseNonNegative, toSavePayload, uniqueSorted, validateRows } from "./portfolioTargets";

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
