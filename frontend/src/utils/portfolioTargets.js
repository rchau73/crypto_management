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

// Keeps the rows whose every filtered column contains the typed text
// (case-insensitive). Notes is the joined source list ("Binance | Ledger"),
// so filtering it by "binance" finds every position held there. A new,
// unsaved row always stays visible: it would vanish while being typed in.
export function filterRows(rows, filters) {
  const active = Object.entries(filters)
    .map(([key, text]) => [key, text.trim().toLowerCase()])
    .filter(([, text]) => text !== "");
  if (active.length === 0) return rows;
  return rows.filter(
    (row) => row.isNew || active.every(([key, text]) => String(row[key] ?? "").toLowerCase().includes(text))
  );
}

const NUMERIC_COLUMNS = new Set(["target_percent", "current_quantity"]);

// Sort value for a column: numbers as numbers (an edited Target % is a
// string), text with null/undefined as "" so they don't sort as "null".
export function sortValue(row, key) {
  if (NUMERIC_COLUMNS.has(key)) return Number(row[key]) || 0;
  return String(row[key] ?? "");
}

export function uniqueSorted(values) {
  return Array.from(new Set(values.filter(Boolean))).sort();
}
