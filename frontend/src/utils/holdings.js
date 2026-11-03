// Rows for the Holdings page, one shape for all four "View by" choices,
// so one table (sorting, filtering, columns) serves them all. Pure: the
// page passes the allocations report in and renders what comes out.
import { aggregateBySymbol, bandStatus } from "./allocationMath";

export const VIEWS = [
  { key: "position", label: "Position", nameLabel: "Asset" },
  { key: "asset", label: "Asset", nameLabel: "Asset" },
  { key: "group", label: "Group", nameLabel: "Group" },
  { key: "barca", label: "BARCA", nameLabel: "BARCA" },
];
export const DEFAULT_VIEW = "asset";

// One DCA step is 30% of the $ gap (BARCA.md rule 3).
export const DCA_FRACTION = 0.3;

function num(value) {
  const n = typeof value === "number" ? value : Number(value);
  return Number.isFinite(n) ? n : 0;
}

function uniqueJoined(values) {
  return Array.from(new Set(values.filter(Boolean))).sort().join(", ");
}

// Measures one row against the whole wallet. Targets are shares of the
// whole wallet, so current % must be too, whatever rows are on screen.
function withMetrics(row, totalValue) {
  const value = num(row.value);
  const targetPercent = num(row.targetPercent);
  const currentPercent = totalValue > 0 ? (value / totalValue) * 100 : 0;
  const gap = value - (totalValue * targetPercent) / 100;
  const stepAmount = Math.abs(gap) * DCA_FRACTION;
  return {
    ...row,
    value,
    targetPercent,
    currentPercent,
    deviation: currentPercent - targetPercent,
    gap,
    status: bandStatus(currentPercent, targetPercent),
    // Above target = sell side, below = buy side. Under a cent is no step.
    step: stepAmount >= 0.01 ? { verb: gap > 0 ? "Sell" : "Buy", amount: stepAmount } : null,
  };
}

// `report`: { allocations, groupAllocations, barcaAllocations,
// barcaActualAllocations } as returned by useAllocations.
export function buildHoldingRows(view, report) {
  const allocations = report.allocations || [];
  const totalValue = allocations.reduce((sum, a) => sum + num(a.value), 0);

  let rows;
  if (view === "position") {
    rows = allocations.map((a, i) => ({
      id: `${a.symbol}|${a.group}|${a.barca}|${i}`,
      name: a.symbol,
      group: a.group,
      barca: a.barca,
      price: num(a.price),
      quantity: num(a.current_quantity),
      value: a.value,
      targetPercent: a.target_percent,
    }));
  } else if (view === "group") {
    rows = (report.groupAllocations || []).map((g) => ({
      id: g.group,
      name: g.group,
      value: g.value,
      targetPercent: g.target_percent,
    }));
  } else if (view === "barca") {
    // Every BARCA with a target, plus any holding value without one.
    const targeted = report.barcaAllocations || [];
    const names = new Set(targeted.map((b) => b.barca));
    const actuals = report.barcaActualAllocations || [];
    rows = [
      ...targeted.map((b) => ({
        id: b.barca,
        name: b.barca,
        value: actuals.find((a) => a.barca === b.barca)?.value ?? b.value,
        targetPercent: b.target_percent,
      })),
      ...actuals
        .filter((a) => !names.has(a.barca))
        .map((a) => ({ id: a.barca, name: a.barca, value: a.value, targetPercent: 0 })),
    ];
  } else {
    // "asset": one row per symbol, across the groups/BARCAs holding it.
    rows = aggregateBySymbol(allocations).map((a) => {
      const positions = allocations.filter((p) => p.symbol === a.symbol);
      return {
        id: a.symbol,
        name: a.symbol,
        group: uniqueJoined(positions.map((p) => p.group)),
        barca: uniqueJoined(positions.map((p) => p.barca)),
        price: a.price,
        quantity: a.current_quantity,
        value: a.value,
        targetPercent: a.target_percent,
      };
    });
  }
  return { rows: rows.map((r) => withMetrics(r, totalValue)), totalValue };
}

// "Contains" text match on the name, group and BARCA, and optionally only
// rows outside their band.
export function filterHoldingRows(rows, { query = "", outOfBandOnly = false } = {}) {
  const q = query.trim().toLowerCase();
  return rows.filter((row) => {
    if (outOfBandOnly && row.status === "in") return false;
    if (!q) return true;
    return [row.name, row.group, row.barca].some((field) => String(field || "").toLowerCase().includes(q));
  });
}
