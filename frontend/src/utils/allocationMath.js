// Pure calculations shared by the allocation tables and the dashboard chart.
// Nothing here touches React state or the DOM, so it's all trivially testable.

export function computeYAxisDomain(rows, keys) {
  if (!Array.isArray(rows) || rows.length === 0) return ["dataMin", "dataMax"];
  if (!Array.isArray(keys) || keys.length === 0) return ["dataMin", "dataMax"];
  let min = Infinity;
  let max = -Infinity;
  rows.forEach((row) => {
    keys.forEach((key) => {
      const raw = row[key];
      const value = typeof raw === "number" ? raw : Number(raw);
      if (isNaN(value)) return;
      if (value < min) min = value;
      if (value > max) max = value;
    });
  });
  if (!isFinite(min) || !isFinite(max)) return ["dataMin", "dataMax"];
  if (min === max) {
    const pad = min === 0 ? 1 : Math.abs(min) * 0.05;
    return [min - pad, max + pad];
  }
  const pad = (max - min) * 0.05;
  return [min - pad, max + pad];
}

export function getTotalTargetPercent(rows) {
  return rows.reduce((sum, row) => {
    const val = typeof row.target_percent === "number" ? row.target_percent : parseFloat(row.target_percent);
    return sum + (isNaN(val) ? 0 : val);
  }, 0);
}

// current %, deviation vs. target, $ deviation, and suggested DCA top-up for
// a single row, given the total value its percentage should be measured
// against (the whole wallet, or a filtered subset of it).
export function deriveRowMetrics(row, totalValue) {
  const targetPercent = row.target_percent || 0;
  const currentPercent = totalValue > 0 ? (row.value / totalValue) * 100 : 0;
  const deviation = currentPercent - targetPercent;
  const targetValue = totalValue * (targetPercent / 100);
  const valueDeviation = row.value - targetValue;
  const dca = Math.abs(valueDeviation) * 0.3;
  return { currentPercent, deviation, valueDeviation, dca };
}

// BARCA rule 1 (BARCA.md): a row needs action only when its current share is
// more than 20% *of its own target* away from that target. Relative on
// purpose: 2 points off is noise for a 50% target but a 20% miss for 10%.
export const BAND_FRACTION = 0.2;

// "high" = outside the band, "normal" = inside it. A row holding value with
// no target at all (0%) is outside any band.
export function bandSeverity(currentPercent, targetPercent) {
  const current = Number(currentPercent) || 0;
  const target = Number(targetPercent) || 0;
  if (target <= 0) return current > 0 ? "high" : "normal";
  return Math.abs(current - target) > target * BAND_FRACTION ? "high" : "normal";
}

// $ gap between a row and its target when only its deviation (in percentage
// points of the whole wallet) is known: those points are of the *total*, not
// of the row's own value. Positive = above target.
export function valueGapFromDeviation(deviationPoints, totalValue) {
  return (totalValue * (Number(deviationPoints) || 0)) / 100;
}

// Collapse per-asset rows (one per symbol+group+barca) into one row per
// symbol, summing quantity/value/target across whichever groups/wallets hold
// that asset.
export function aggregateBySymbol(rows) {
  const bySymbol = new Map();
  rows.forEach((row) => {
    const symbol = row.symbol;
    if (!symbol) return;
    const qty = typeof row.current_quantity === "number" ? row.current_quantity : Number(row.current_quantity || 0);
    const value = typeof row.value === "number" ? row.value : Number(row.value || 0);
    const target = typeof row.target_percent === "number" ? row.target_percent : Number(row.target_percent || 0);
    const price = typeof row.price === "number" ? row.price : Number(row.price || 0);

    let entry = bySymbol.get(symbol);
    if (!entry) {
      entry = { symbol, current_quantity: 0, value: 0, target_percent: 0, last_price: 0 };
      bySymbol.set(symbol, entry);
    }
    entry.current_quantity += isNaN(qty) ? 0 : qty;
    entry.value += isNaN(value) ? 0 : value;
    entry.target_percent += isNaN(target) ? 0 : target;
    if (!isNaN(price) && price > 0) entry.last_price = price;
  });

  return Array.from(bySymbol.values()).map((entry) => ({
    ...entry,
    price: entry.current_quantity > 0 ? entry.value / entry.current_quantity : entry.last_price || 0,
  }));
}
