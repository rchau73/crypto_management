// Pure helpers for the Portfolio Targets editor — no React, easy to test.
import { getTotalTargetPercent } from "./allocationMath";

export const ASSET_CLASSES = ["crypto", "br-equities", "us-indices"];

// Floats that should add up to 100 (e.g. 33.34 + 33.33 + 33.33) — same
// tolerance the backend uses.
export const SUM_TOLERANCE = 0.01;

export function blankRow() {
  return {
    symbol: "",
    group_name: "",
    barca: "",
    asset_class: "crypto",
    target_percent: 0,
    current_quantity: 0,
    notes: "",
    isNew: true,
  };
}

// Returns the number typed in `value`, or null if it isn't a number >= 0.
// ("" is null too — an empty box is not the same as 0.)
export function parseNonNegative(value) {
  if (value === "" || value === null || value === undefined) return null;
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? number : null;
}

function isValidPercent(value) {
  const number = parseNonNegative(value);
  return number !== null && number <= 100;
}

// Only a position backed by exactly one ledger source can have its quantity
// corrected: with several sources there's no way to know which one the new
// number belongs to (the backend refuses it too).
export function canCorrectQuantity(row) {
  return !row.isNew && row.source_count === 1;
}

export function validateRows(rows) {
  const sum = getTotalTargetPercent(rows);
  const everyRowValid = rows.every(
    (r) =>
      r.symbol.trim() !== "" &&
      isValidPercent(r.target_percent) &&
      (!r.isNew || parseNonNegative(r.current_quantity) !== null)
  );
  const isValid = rows.length > 0 && everyRowValid && Math.abs(sum - 100) <= SUM_TOLERANCE;
  return { sum, isValid };
}

// What "Save All" sends. An existing row never sends quantity/notes: the
// table shows a SUM over several sources, and writing that sum back would
// double the holding. Only a brand-new row carries a starting quantity.
export function toSavePayload(rows) {
  return rows.map((r) => {
    const row = {
      symbol: r.symbol.trim(),
      group_name: r.group_name?.trim() || null,
      barca: r.barca?.trim() || null,
      asset_class: r.asset_class,
      target_percent: Number(r.target_percent),
    };
    if (r.isNew) {
      row.current_quantity = parseNonNegative(r.current_quantity) ?? 0;
      row.notes = r.notes?.trim() || null;
    }
    return row;
  });
}

export function uniqueSorted(values) {
  return Array.from(new Set(values.filter(Boolean))).sort();
}
